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

