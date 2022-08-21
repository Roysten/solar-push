use chrono::TimeZone;
use chrono_tz::Europe::Amsterdam;
use clap::Parser;
use curl::easy::{Easy, List};
use rusqlite::types::Value;
use rusqlite::Connection;

use solar_data::SolarData;

mod solar_data;

const SELECT_SQL: &str = "
SELECT
    id,
    device_id,
    tracker_id,
    timestamp,
    energy_generation,
    power_generation,
    temperature,
    voltage,
    uploaded
FROM solar
WHERE
    uploaded = 0
    AND device_id = ?
    AND tracker_id = ?
ORDER BY id
LIMIT 30";

const UPDATE_SQL: &str = "
UPDATE
    solar
SET
    uploaded = 1
WHERE
    id IN rarray(?)";

#[derive(Debug)]
enum AppError {
    Curl(curl::Error),
    CurlForm(curl::FormError),
    Sqlite(rusqlite::Error),
}

impl From<curl::Error> for AppError {
    fn from(e: curl::Error) -> Self {
        Self::Curl(e)
    }
}

impl From<curl::FormError> for AppError {
    fn from(e: curl::FormError) -> Self {
        Self::CurlForm(e)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

struct Tracker {
    device_id: u8,
    array_id: u8,
    system_id: &'static str,
}

#[derive(Parser)]
#[clap(version)]
struct Args {
    #[clap()]
    db_path: String,
}

fn build_post_data(samples: &Vec<SolarData>) -> String {
    let mut data_string = "c1=2&data=".to_owned();
    for sample in samples {
        let datetime = Amsterdam.timestamp(sample.timestamp as i64, 0);
        let sample_data_string = [
            &datetime.format("%Y%m%d").to_string(),
            &datetime.format("%H:%M").to_string(),
            // &sample.energy_generation.to_string(),
            "-1",
            &sample.power_generation.to_string(),
            "-1",
            "-1",
            &sample.temperature.to_string(),
            &sample.voltage.to_string(),
        ]
        .join(",");
        data_string += &(sample_data_string + ";");
    }
    data_string
}

fn send_request(easy: &mut Easy, samples: &Vec<SolarData>, system_id: &str) -> Result<(), AppError> {
    easy.url("https://pvoutput.org/service/r2/addbatchstatus.jsp")?;
    easy.post(true)?;
    let post_data = build_post_data(&samples);
    easy.post_fields_copy(&post_data.as_bytes())?;
    let mut headers = List::new();
    headers.append(&format!("X-Pvoutput-SystemId: {}", system_id))?;
    headers.append(&format!(
        "X-Pvoutput-Apikey: {}",
        include_str!("../apikey.txt")
    ))?;
    easy.http_headers(headers)?;
    /*easy.write_function(|x| {
        Ok(std::io::stdout().write(x).unwrap())
    });*/
    easy.perform()?;
    println!("HTTP {}", easy.response_code()?);
    Ok(())
}

fn get_samples(db: &Connection, tracker: &Tracker) -> Result<Vec<SolarData>, AppError> {
    let mut query = db.prepare(SELECT_SQL)?;
    let samples = query
        .query_map((tracker.device_id, tracker.array_id), |row| {
            Ok(SolarData {
                id: row.get(0)?,
                device_id: row.get(1)?,
                tracker_id: row.get(2)?,
                timestamp: row.get(3)?,
                energy_generation: row.get(4)?,
                power_generation: row.get(5)?,
                temperature: row.get(6)?,
                voltage: row.get(7)?,
            })
        })?
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    Ok(samples)
}

fn main() -> Result<(), AppError> {
    let args = Args::parse();
    let db = Connection::open(args.db_path)?;
    rusqlite::vtab::array::load_module(&db)?;

    let trackers = vec![
        Tracker {
            device_id: 2,
            array_id: 1,
            system_id: "92309",
        },
        Tracker {
            device_id: 2,
            array_id: 2,
            system_id: "92748",
        },
        Tracker {
            device_id: 3,
            array_id: 1,
            system_id: "92869",
        },
    ];

    let mut easy = Easy::new();
    for tracker in trackers {
        loop {
            let samples = get_samples(&db, &tracker)?;
            if samples.is_empty() {
                break;
            }

            send_request(&mut easy, &samples, &tracker.system_id)?;
            let ids_sent = samples
                .iter()
                .map(|s| Value::Integer(s.id as i64))
                .collect::<Vec<_>>();
            let mut query = db.prepare(UPDATE_SQL)?;
            query.execute([std::rc::Rc::new(ids_sent)])?;
        }
    }

    Ok(())
}
