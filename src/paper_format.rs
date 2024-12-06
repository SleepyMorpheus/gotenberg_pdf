use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

// Paper Format
// ------------
#[derive(Debug, Clone, PartialEq)]
pub enum PaperFormat {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    Ledger,
    Legal,
    Letter,
    Tabloid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaperSize {
    size: f64,
    unit: Option<Unit>,
}

impl PaperSize {
    pub fn new(size: f64, unit: Unit) -> Self {
        PaperSize {
            size,
            unit: Some(unit),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Mm,
    Cm,
    In,
    Px,
    Pt,
    Pc,
}

impl fmt::Display for PaperSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unit {
            Some(Unit::Mm) => write!(f, "{}mm", self.size),
            Some(Unit::Cm) => write!(f, "{}cm", self.size),
            Some(Unit::In) => write!(f, "{}in", self.size),
            Some(Unit::Px) => write!(f, "{}px", self.size),
            Some(Unit::Pt) => write!(f, "{}pt", self.size),
            Some(Unit::Pc) => write!(f, "{}pc", self.size),
            None => write!(f, "{}", self.size),
        }
    }
}

impl FromStr for PaperSize {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let size = s.trim_end_matches(|c: char| !c.is_digit(10) && c != '.');
        let unit = s.trim_start_matches(|c: char| c.is_digit(10) || c == '.');

        let size = size.parse::<f64>().map_err(|_| "Invalid size")?;
        let unit = match unit {
            "mm" => Some(Unit::Mm),
            "cm" => Some(Unit::Cm),
            "in" => Some(Unit::In),
            "px" => Some(Unit::Px),
            "pt" => Some(Unit::Pt),
            "pc" => Some(Unit::Pc),
            "" => None,
            _ => return Err("Invalid unit".to_string()),
        };

        Ok(PaperSize { size, unit })
    }
}

impl PaperFormat {
    pub fn height(&self) -> PaperSize {
        match self {
            PaperFormat::A0 => PaperSize::new(46.8, Unit::Cm),
            PaperFormat::A1 => PaperSize::new(33.1, Unit::Cm),
            PaperFormat::A2 => PaperSize::new(23.4, Unit::Cm),
            PaperFormat::A3 => PaperSize::new(16.54, Unit::Cm),
            PaperFormat::A4 => PaperSize::new(11.7, Unit::In),
            PaperFormat::A5 => PaperSize::new(8.27, Unit::In),
            PaperFormat::A6 => PaperSize::new(5.83, Unit::In),
            PaperFormat::Ledger => PaperSize::new(11.0, Unit::In),
            PaperFormat::Legal => PaperSize::new(14.0, Unit::In),
            PaperFormat::Letter => PaperSize::new(11.0, Unit::In),
            PaperFormat::Tabloid => PaperSize::new(17.0, Unit::In),
        }
    }

    pub fn width(&self) -> PaperSize {
        match self {
            PaperFormat::A0 => PaperSize::new(33.1, Unit::Cm),
            PaperFormat::A1 => PaperSize::new(23.4, Unit::Cm),
            PaperFormat::A2 => PaperSize::new(16.54, Unit::Cm),
            PaperFormat::A3 => PaperSize::new(11.7, Unit::Cm),
            PaperFormat::A4 => PaperSize::new(8.27, Unit::In),
            PaperFormat::A5 => PaperSize::new(5.83, Unit::In),
            PaperFormat::A6 => PaperSize::new(4.13, Unit::In),
            PaperFormat::Ledger => PaperSize::new(17.0, Unit::In),
            PaperFormat::Legal => PaperSize::new(8.5, Unit::In),
            PaperFormat::Letter => PaperSize::new(8.5, Unit::In),
            PaperFormat::Tabloid => PaperSize::new(11.0, Unit::In),
        }
    }
}

impl fmt::Display for PaperFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaperFormat::A0 => write!(f, "A0"),
            PaperFormat::A1 => write!(f, "A1"),
            PaperFormat::A2 => write!(f, "A2"),
            PaperFormat::A3 => write!(f, "A3"),
            PaperFormat::A4 => write!(f, "A4"),
            PaperFormat::A5 => write!(f, "A5"),
            PaperFormat::A6 => write!(f, "A6"),
            PaperFormat::Ledger => write!(f, "Ledger"),
            PaperFormat::Legal => write!(f, "Legal"),
            PaperFormat::Letter => write!(f, "Letter"),
            PaperFormat::Tabloid => write!(f, "Tabloid"),
        }
    }
}

impl FromStr for PaperFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A0" => Ok(PaperFormat::A0),
            "A1" => Ok(PaperFormat::A1),
            "A2" => Ok(PaperFormat::A2),
            "A3" => Ok(PaperFormat::A3),
            "A4" => Ok(PaperFormat::A4),
            "A5" => Ok(PaperFormat::A5),
            "A6" => Ok(PaperFormat::A6),
            "Ledger" => Ok(PaperFormat::Ledger),
            "Legal" => Ok(PaperFormat::Legal),
            "Letter" => Ok(PaperFormat::Letter),
            "Tabloid" => Ok(PaperFormat::Tabloid),
            _ => Err("Invalid paper format".to_string()),
        }
    }
}

// Custom Serializer for PaperSize
impl Serialize for PaperSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

// Custom Deserializer for PaperSize
impl<'de> Deserialize<'de> for PaperSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_paper_size_from_str_valid() {
        assert_eq!(
            "11.7in".parse::<PaperSize>(),
            Ok(PaperSize::new(11.7, Unit::In))
        );
        assert_eq!(
            "33.1cm".parse::<PaperSize>(),
            Ok(PaperSize::new(33.1, Unit::Cm))
        );
        assert_eq!(
            "5.83in".parse::<PaperSize>(),
            Ok(PaperSize::new(5.83, Unit::In))
        );
        assert_eq!(
            "5in".parse::<PaperSize>(),
            Ok(PaperSize::new(5.0, Unit::In))
        );
    }

    #[test]
    fn test_paper_size_from_str_invalid() {
        assert!("abc".parse::<PaperSize>().is_err());
        assert!("11.7invalid".parse::<PaperSize>().is_err());
    }

    #[test]
    fn test_paper_size_to_string() {
        let size = PaperSize::new(11.7, Unit::In);
        assert_eq!(size.to_string(), "11.7in");

        let size = PaperSize::new(33.1, Unit::Cm);
        assert_eq!(size.to_string(), "33.1cm");
    }

    #[test]
    fn test_paper_format_from_str_valid() {
        assert_eq!("A4".parse::<PaperFormat>(), Ok(PaperFormat::A4));
        assert_eq!("Ledger".parse::<PaperFormat>(), Ok(PaperFormat::Ledger));
    }

    #[test]
    fn test_paper_format_from_str_invalid() {
        assert!("Invalid".parse::<PaperFormat>().is_err());
        assert!("".parse::<PaperFormat>().is_err());
    }

    #[test]
    fn test_paper_format_dimensions() {
        let a4 = PaperFormat::A4;
        assert_eq!(a4.height(), PaperSize::new(11.7, Unit::In));
        assert_eq!(a4.width(), PaperSize::new(8.27, Unit::In));
    }

    #[test]
    fn test_paper_size_serialization() {
        let size = PaperSize::new(11.7, Unit::In);
        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, "\"11.7in\"");
    }

    #[test]
    fn test_paper_size_deserialization() {
        let serialized = "\"11.7in\"";
        let deserialized: PaperSize = serde_json::from_str(serialized).unwrap();
        assert_eq!(deserialized, PaperSize::new(11.7, Unit::In));
    }

    #[test]
    fn test_paper_format_display() {
        assert_eq!(PaperFormat::A4.to_string(), "A4");
        assert_eq!(PaperFormat::Ledger.to_string(), "Ledger");
    }
}
