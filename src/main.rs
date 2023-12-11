use std::error::Error;
use std::fs::{File, OpenOptions};
use std::time::{SystemTime, UNIX_EPOCH};

use csv::{ReaderBuilder, WriterBuilder};
use chrono::prelude::{Local, DateTime};
use chrono::Datelike;

#[derive(Debug)]
struct Invoice {
    name: String,
    date: u128,
    days_worked: u8,
    daily_rate: f64,
    currency: String,
    gross_profit: f64,
    net_profit: f64,
    government_tax: f64,
    social_contribution_tax: f64,
    total_tax: f64
}

#[derive(Debug)]
struct TaxBucket {
    to: Option<u32>,
    perc: f64
}

impl Invoice {
    fn file_path() -> String {
        let local: DateTime<Local> = Local::now();
        let current_year = local.year();
        let file_path = format!("src/invoices_{}.csv", current_year);

        file_path
    }

    fn write_invoice_to_csv(invoice:Invoice) {
        let file_path = Self::file_path();

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file_path).unwrap();

        let mut writer = WriterBuilder::new().from_writer(file);

        let record = vec![
            invoice.name,
            invoice.date.to_string(),
            invoice.days_worked.to_string(),
            invoice.daily_rate.to_string(),
            invoice.currency,
            invoice.gross_profit.to_string(),
            invoice.net_profit.to_string(),
            invoice.government_tax.to_string(),
            invoice.social_contribution_tax.to_string(),
            invoice.total_tax.to_string(),
        ];

        let _ = writer.write_record(&record);
        let empty_slice: &[&str] = &[];
        let _ = writer.write_record(empty_slice);
        let _ = writer.flush();
    }

    fn fetch_invoices() -> Result<Vec<Invoice>, Box<dyn Error>> {
        let file_path = Self::file_path();
        let mut invoices: Vec<Invoice> = Vec::new();

        if !std::path::Path::new(&file_path).exists() {
            let file = File::create(&file_path)?;
            
            let headers = vec![
                "name", "date", "days_worked", "daily_rate", "currency",
                "gross_profit", "net_profit", "government_tax",
                "social_contribution_tax", "total_tax"
            ];
            let mut writer = WriterBuilder::new().from_writer(file);
            writer.write_record(&headers)?;
            return Ok(invoices);
        }

        let file = File::open(file_path)?;
        let mut reader = ReaderBuilder::new().from_reader(file);

        for result in reader.records() {
            let record = result?;

            let invoice = Invoice {
                name: record[0].to_string(),
                date: record[1].parse().unwrap(),
                days_worked: record[2].parse().unwrap(),
                daily_rate: record[3].parse().unwrap(),
                currency: record[4].to_string(),
                gross_profit: record[5].parse().unwrap(),
                net_profit: record[6].parse().unwrap(),
                government_tax: record[7].parse().unwrap(),
                social_contribution_tax: record[8].parse().unwrap(),
                total_tax: record[9].parse().unwrap(),
            };
    
            invoices.push(invoice);
        };

        Ok(invoices)
    }

    fn tax_buckets() -> Vec<TaxBucket> {
        vec![
            TaxBucket {
                to: Some(13_870),
                perc: 0.25
            },
            TaxBucket {
                to: Some(24_480),
                perc: 0.40
            },
            TaxBucket {
                to: Some(42_370),
                perc: 0.45
            },
            TaxBucket {
                to: None,
                perc: 0.5
            }
        ]
    }

    fn appliable_tax_buckets(total_gross_profit:f64, mut gross_profit:f64) -> Vec<(f64, f64)> {
        let mut appliable_tax_buckets = vec![];

        let gross_profit_range = (total_gross_profit, total_gross_profit + &gross_profit);

        let tax_buckets = Self::tax_buckets();
        for tax_bucket in tax_buckets {
            if tax_bucket.to == None {
                appliable_tax_buckets.push((gross_profit, tax_bucket.perc));
                return appliable_tax_buckets;
            } else if gross_profit_range.0 > tax_bucket.to.unwrap() as f64 {
                continue;
            } else {
                if gross_profit_range.1 < tax_bucket.to.unwrap() as f64 {
                    appliable_tax_buckets.push((gross_profit, tax_bucket.perc));
                    return appliable_tax_buckets;
                } else {
                    let diff = tax_bucket.to.unwrap() as f64 - gross_profit_range.1;
                    appliable_tax_buckets.push((diff, tax_bucket.perc));
                    gross_profit -= diff;
                }
            }
        }
        appliable_tax_buckets
    }

    fn calc_government_tax(appliable_tax_buckets: Vec<(f64, f64)>) -> (f64, f64) {
        let mut profit_after_government_tax = 0.0;
        let mut government_tax = 0.0;

        for (gross_profit, tax) in appliable_tax_buckets {
            government_tax += gross_profit * tax;
            profit_after_government_tax += gross_profit - government_tax;
        }

        (profit_after_government_tax, government_tax)
    }

    fn calc_social_contribution(profit_after_government_tax: f64) -> (f64, f64) {
        const SOCIAL_CONTRIBUTION_FEE: f64 = 0.205;
        
        let social_contribution = profit_after_government_tax * SOCIAL_CONTRIBUTION_FEE;
        let net_profit = profit_after_government_tax - social_contribution;
        
        (net_profit, social_contribution)
    }

    fn calc_taxes(days_worked:u8, daily_rate:f64, invoices:Vec<Invoice>) -> (f64, f64, f64, f64) {
        let total_gross_profit: f64 = invoices.iter().map(|record| record.gross_profit).sum();
        let gross_profit = days_worked as f64 * daily_rate;

        let appliable_tax_buckets = Self::appliable_tax_buckets(total_gross_profit, gross_profit);

        let (profit_after_government_tax, government_tax) = Self::calc_government_tax(appliable_tax_buckets);
        let (net_profit, social_contribution) = Self::calc_social_contribution(profit_after_government_tax);

        (gross_profit, net_profit, government_tax, social_contribution)
    }

    pub fn new(name:String, days_worked:u8, daily_rate:Option<f64>, currency:Option<String>) {
        const DAILY_RATE: f64 = 500.0;
        const CURRENCY: &str = "EUR";

        let invoices: Vec<Invoice> = Self::fetch_invoices().unwrap();
        
        if invoices.iter().any(|invoice| invoice.name == name) {
            panic!("`name` needs to be unique from other invoices");
        } else if days_worked == 0 {
            panic!("`days_worked` can not be 0");
        } else if daily_rate == Some(0.0) {
            panic!("`daily_rate` can not be 0.0");
        }

        let daily_rate = daily_rate.unwrap_or(DAILY_RATE);
        let currency = currency.unwrap_or(CURRENCY.to_string()).to_uppercase();
        // TODO: If currency != EUR, convert `daily_rate` to EUR

        let (gross_profit, net_profit, government_tax, social_contribution_tax) =
            Self::calc_taxes(days_worked, daily_rate, invoices);

        let current_timestamp= SystemTime::now();
        let since_the_epoch = current_timestamp .duration_since(UNIX_EPOCH).expect("Time went backwards");
        let timestamp_millis  = since_the_epoch.as_millis();
        
        let invoice = Invoice {
            name,
            date: timestamp_millis,
            days_worked,
            daily_rate,
            currency,
            gross_profit,
            net_profit,
            government_tax,
            social_contribution_tax,
            total_tax: government_tax + social_contribution_tax
        };
        
        Self::write_invoice_to_csv(invoice);
    }

    pub fn accountant_info() {
        let invoices: Vec<Invoice> = Self::fetch_invoices().unwrap();

        let total_gross_profit: f64 = invoices.iter().map(|record| record.gross_profit).sum();
        let total_net_profit: f64 = invoices.iter().map(|record| record.net_profit).sum();
        let total_gov_tax: f64 = invoices.iter().map(|record| record.government_tax).sum();
        let total_social_contribution: f64 = invoices.iter().map(|record| record.social_contribution_tax).sum();
        let total_tax: f64 = invoices.iter().map(|record| record.total_tax).sum();

        println!("Total gross profit: {}", total_gross_profit);
        println!("Total net profit: {}", total_net_profit);
        println!("Total government tax: {}", total_gov_tax);
        println!("Total social contribution: {}", total_social_contribution);
        println!("Total taxes: {}", total_tax);
    }
}

// TODO: Turn into a CLI

fn main() {
    Invoice::new("test_invoice_1".to_string(), 5, Some(500.0), Some("EUR".to_string()));
    Invoice::accountant_info();
}