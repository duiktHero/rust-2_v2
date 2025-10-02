
// Part 2: Safer time handling with Option<Result> and pattern matching.
// We model a device with optional local_time parsed from string.
// We expose a method to render local_time, or return a descriptive error.

#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub local_time: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TimeError {
    #[error("local time is missing")]
    Missing,
    #[error("invalid local time format: {0}")]
    Invalid(String),
}

impl Device {
    pub fn parsed_local_time_minutes(&self) -> Result<i64, TimeError> {
        let Some(ref s) = self.local_time else { return Err(TimeError::Missing) };
        let parts: Vec<&str> = s.split(|c| c==' ' || c==':' || c=='-').collect();
        if parts.len() < 5 { return Err(TimeError::Invalid(s.clone())); }
        let hh = parts[3].parse::<i64>().map_err(|_| TimeError::Invalid(s.clone()))?;
        let mm = parts[4].parse::<i64>().map_err(|_| TimeError::Invalid(s.clone()))?;
        Ok(hh*60 + mm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ok_parse() {
        let d = Device { id: "A".into(), local_time: Some("2025-09-30 21:45".into()) };
        assert_eq!(d.parsed_local_time_minutes().unwrap(), 21*60+45);
    }
    #[test]
    fn missing() {
        let d = Device { id: "B".into(), local_time: None };
        assert!(matches!(d.parsed_local_time_minutes(), Err(TimeError::Missing)));
    }
    #[test]
    fn invalid() {
        let d = Device { id: "C".into(), local_time: Some("oops".into()) };
        assert!(matches!(d.parsed_local_time_minutes(), Err(TimeError::Invalid(_))));
    }
}
