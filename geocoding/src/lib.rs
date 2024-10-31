mod types;

include!(concat!(env!("OUT_DIR"), "/admin1CodesASCII.rs"));
include!(concat!(env!("OUT_DIR"), "/admin2Codes.rs"));
include!(concat!(env!("OUT_DIR"), "/countries.rs"));

use types::Country;

pub fn get_country(iso_code: &str) -> Option<&Country> {
    COUNTRIES.get(iso_code)
}

pub fn get_admin1_code(key: &str) -> Option<&'static str> {
    ADMIN_1_CODES.get(key).copied()
}

pub fn get_admin2_code(key: &str) -> Option<&'static str> {
    ADMIN_2_CODES.get(key).copied()
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
