use chrono::prelude::Utc;
use chrono::DateTime;
use reqwest::Client;

use serde::Deserialize;
use std::time::Duration;
use std::{error::Error, time::SystemTime};

const DEBUG: bool = true;
const PLACE_SLEEP: Duration = Duration::from_millis(100);
const CENTER_SLEEP: Duration = Duration::from_millis(100);

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.212 Safari/537.36";

#[derive(Deserialize, Debug, Clone)]
struct BookingCenter {
    data: BookingData,
}

#[derive(Deserialize, Debug, Clone)]
struct BookingData {
    visit_motives: Vec<VisitMotive>,
    places: Vec<Place>,
    agendas: Vec<Agenda>,
}

#[derive(Deserialize, Debug, Clone)]
struct VisitMotive {
    id: u32,
    name: String,
}
#[derive(Deserialize, Debug, Clone)]
struct Place {
    id: String,
    address: String,
    zipcode: String,
    city: String,
    formal_name: String,
    full_address: String,
    practice_ids: Vec<u32>,
}

#[derive(Deserialize, Debug, Clone)]
struct Agenda {
    id: u32,
    booking_disabled: bool,
    booking_temporary_disabled: bool,
    visit_motive_ids: Vec<u32>,
    practice_id: u32,
}

#[derive(Deserialize, Debug, Clone)]
struct Availabilities {
    total: u32,
    reason: String,
    message: String,
    number_future_vaccinations: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::builder().build()?;

    let centers = vec![
        "centre-de-vaccination-covid-19-ville-de-paris",
        "centre-covid19-paris-5",
        "centre-de-vaccination-covid-19-mairie-du-6eme-arrondissement-de-paris",
        "centre-de-vaccination-mairie-du-7eme-paris",
        "centre-de-vaccination-covid-19-paris-8e",
        "centre-de-vaccination-covid-mairie-du-9eme-arrondissement",
        "centre-de-vaccination-paris-14e",
        "centre-de-vaccination-covid-paris-15e",
        "vaccinodrome-covid-19-porte-de-versailles",
        "centre-de-vaccination-covid-19-mairie-du-16eme-arrondissement",
        "centre-de-vaccination-covid-19-paris-17eme",
        "centre-de-vaccination-covid-19-stade-de-france",
    ];
    // let centers = vec!["vaccinodrome-covid-19-porte-de-versailles"];

    for center in centers {
        check_center(&client, center).await?;
        tokio::time::sleep(CENTER_SLEEP).await;
    }

    Ok(())
}

fn iso_date() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    format!("{}", now.format("%+"))
}

async fn check_center(
    client: &Client,
    center_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = get_center_data(client, center_name).await?;

    let motives = res
        .data
        .visit_motives
        .iter()
        .filter(|motive| {
            motive.name.contains("1re injection") && !motive.name.contains("AstraZeneca")
        })
        .collect::<Vec<_>>();


    if DEBUG {
        println!(
            "\nChecking center: {} found motives: {:?}",
            center_name,
            motives
                .iter()
                .map(|motive| &motive.name)
                .collect::<Vec<_>>()
        );
    }

    if motives.is_empty() {
        println!("No motives found.");
        return Ok(());
    }

    for place in &res.data.places {
        let practice_id = place.practice_ids.get(0).cloned().unwrap_or(0);
        let agendas = res
            .data
            .agendas
            .iter()
            .filter(|agenda| agenda.practice_id == practice_id && !agenda.booking_disabled)
            .filter(|agenda| {
                motives
                    .iter()
                    .any(|motive| agenda.visit_motive_ids.contains(&motive.id))
            })
            .collect::<Vec<_>>();

        if agendas.is_empty() {
            println!("No agendas found!");
            continue;
        }

        // println!("Agendas: {:#?}", agendas);

        let visit_motive_ids = motives.iter().map(|motive| motive.id).collect::<Vec<_>>();
        let agenda_ids = agendas.iter().map(|agenda| agenda.id).collect::<Vec<_>>();
        let practice_ids = vec![practice_id];

        let availabilities =
            get_availability(client, visit_motive_ids, agenda_ids, practice_ids).await?;

        if availabilities.total > 0 {
            println!(
                "FOUND AVAILABLE SLOTS. Message={} Place={} Zip={} Address={}",
                availabilities.message, place.formal_name, place.zipcode, place.address
            );
        } else {
            if DEBUG {
                println!(
                    "No available slots. Reason={} Place={} Zip={} Address={}",
                    availabilities.message, place.formal_name, place.zipcode, place.address
                );
            }
        }

        tokio::time::sleep(PLACE_SLEEP).await;
    }
    Ok(())
}

async fn get_center_data(
    client: &Client,
    center_name: &str,
) -> Result<BookingCenter, Box<dyn Error>> {
    let url = format!("https://www.doctolib.fr/booking/{}.json", center_name);
    let res = client
        .get(&url)
        .header("authority", "www.doctolib.fr")
        .header("user-agent", USER_AGENT)
        .header("accept", "text/json")
        .send()
        .await?;

    let res = res.json::<BookingCenter>().await?;
    Ok(res)
}

fn vec_to_param(vec: &Vec<u32>) -> String {
    let strs = vec.iter().map(|val| val.to_string()).collect::<Vec<_>>();
    strs.join(",")
}

async fn get_availability(
    client: &Client,
    visit_motive_ids: Vec<u32>,
    agenda_ids: Vec<u32>,
    practice_ids: Vec<u32>,
) -> Result<Availabilities, Box<dyn Error>> {
    let url = "https://www.doctolib.fr/availabilities.json";
    let start_date = iso_date();
    let params = [
        ("start_date", start_date),
        ("visit_motive_ids", vec_to_param(&visit_motive_ids)),
        ("agenda_ids", vec_to_param(&agenda_ids)),
        ("practice_ids", vec_to_param(&practice_ids)),
        ("insurance_sector", "public".to_owned()),
        ("destroy_temporary", "true".to_owned()),
        ("limit", "2".to_owned()),
    ];
    let res = client
        .get(url)
        .form(&params)
        .header("authority", "www.doctolib.fr")
        .header("user-agent", USER_AGENT)
        .header("accept", "text/json")
        .send()
        .await?;

    let res = if !res.status().is_success() {
        println!("Was not a success!");
        println!("PARAMS: {:?}", &params);
        println!("RAW: {:?}", res);
        let res = res.text().await?;
        println!("BODY: {:?}", res);
        Availabilities {
            total: 0,
            reason: "Error performing request".to_owned(),
            message: "Error performing request".to_owned(),
            number_future_vaccinations: 0,
        }
    } else {
        res.json::<Availabilities>().await?
    };

    Ok(res)
}
