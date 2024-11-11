use phf::{Map, phf_map};

include!(concat!(env!("OUT_DIR"), "/admin1CodesASCII.rs"));
include!(concat!(env!("OUT_DIR"), "/admin2Codes.rs"));
include!(concat!(env!("OUT_DIR"), "/countries.rs"));

pub fn get_country(iso_code: &str) -> Option<&Country> {
    COUNTRIES.get(iso_code)
}

pub fn get_admin1_code(key: &str) -> Option<&'static str> {
    ADMIN_1_CODES.get(key).copied()
}

pub fn get_admin2_code(key: &str) -> Option<&'static str> {
    ADMIN_2_CODES.get(key).copied()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

impl Continent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Continent::Africa => "AF",
            Continent::Asia => "AS",
            Continent::Europe => "EU",
            Continent::NorthAmerica => "NA",
            Continent::Oceania => "OC",
            Continent::SouthAmerica => "SA",
            Continent::Antarctica => "AN",
        }
    }
}

impl TryFrom<&str> for Continent {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "AF" => Ok(Continent::Africa),
            "AS" => Ok(Continent::Asia),
            "EU" => Ok(Continent::Europe),
            "NA" => Ok(Continent::NorthAmerica),
            "OC" => Ok(Continent::Oceania),
            "SA" => Ok(Continent::SouthAmerica),
            "AN" => Ok(Continent::Antarctica),
            _ => Err(format!("Invalid continent code {value}")),
        }
    }
}

/// Represents a country as read from the file `countryInfo.txt`.
#[derive(Debug, Clone)]
pub struct Country {
    pub iso_code: &'static str,
    pub name: &'static str,
    pub continent: Continent,
}

impl PartialEq for Country {
    fn eq(&self, other: &Self) -> bool {
        self.iso_code == other.iso_code
    }
}

impl Eq for Country {}

impl std::hash::Hash for Country {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.iso_code.hash(hasher);
    }
}

/// Represents a place as read from the file 'allCountries.txt'.
#[derive(Debug, Clone)]
pub struct Place {
    pub name: &'static str,
    pub lat: f32,
    pub lon: f32,
    pub iso_code: &'static str,
    pub admin1: &'static str,
    pub admin2: &'static str,
    pub timezone: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_country() {
        let result = get_country("DK").unwrap();
        assert_eq!(result.name, "Denmark");
        assert_eq!(result.continent, Continent::Europe);
    }

    #[test]
    fn test_get_admin1_code() {
        let result = get_admin1_code("GB.ENG").unwrap();
        assert_eq!(result, "England");
    }

    #[test]
    fn test_get_admin2_code() {
        let result = get_admin2_code("GB.ENG.J9").unwrap();
        assert_eq!(result, "Nottinghamshire");
    }
}
