#[derive(Debug, Copy, Clone)]
pub enum Continent {
    // Code = AF
    Africa,
    // Code = AS
    Asia,
    // Code = EU
    Europe,
    // Code = NA
    NorthAmerica,
    // Code = OC
    Oceania,
    // Code = SA
    SouthAmerica,
    // Code = AN
    Antarctica,
}

impl TryFrom<&str> for Continent {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "af" => Ok(Continent::Africa),
            "as" => Ok(Continent::Asia),
            "eu" => Ok(Continent::Europe),
            "na" => Ok(Continent::NorthAmerica),
            "oc" => Ok(Continent::Oceania),
            "sa" => Ok(Continent::SouthAmerica),
            "an" => Ok(Continent::Antarctica),
            _ => Err(format!("Invalid continent code {value}")),
        }
    }
}

/// Represents a country as read from the file `countryInfo.txt`.
#[derive(Debug, Clone)]
pub struct Country {
    pub iso_code: String,
    pub name: String,
    pub continent: Continent,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub geoname_id: u32,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub country: Country,
    pub admin1: String,
    pub admin2: String,
    pub timezone: String,
}
