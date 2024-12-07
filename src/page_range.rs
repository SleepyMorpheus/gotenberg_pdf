use super::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Represents a set of page ranges, eg `"1,3-5,7"`.
///
/// `PageRange` is designed to handle user input for printing or pagination
/// purposes. It supports single pages, ranges of pages, and combinations
/// of both, expressed in a comma-separated string format (e.g., `"1,3-5,7"`).
///
/// # Example
///
/// ```
/// use gotenberg_pdf::PageRange;
///
/// let range: PageRange = "1,3-5,7".parse().unwrap();
/// assert!(range.in_range(3)); // true
/// assert!(!range.in_range(6)); // false
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PageRange {
    /// A vector of `PageRangeChunk` values (e.g., single pages or ranges).
    chunks: Vec<PageRangeChunk>,
}

impl FromStr for PageRange {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(PageRange { chunks: vec![] });
        }

        let chunks = s
            .split(',')
            .map(str::trim)
            .map(PageRangeChunk::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PageRange { chunks })
    }
}

impl fmt::Display for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted_chunks = self
            .chunks
            .iter()
            .map(PageRangeChunk::to_string)
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{}", formatted_chunks)
    }
}

impl PageRange {
    /// Creates a new `PageRange` from a vector of `PageRangeChunk` values.
    pub fn new(chunks: Vec<PageRangeChunk>) -> Self {
        Self { chunks }
    }

    /// Checks if the given page number is within the page range.
    pub fn in_range(&self, page: usize) -> bool {
        // Empty range means all pages are included
        if self.chunks.is_empty() {
            return true;
        }

        self.chunks.iter().any(|chunk| chunk.in_range(page))
    }
}

/// Represents a chunk of a page range, either a single page or a range of pages.
///
/// # Examples
///
/// ```
/// use gotenberg_pdf::PageRangeChunk;
///
/// let single = PageRangeChunk::SingleValue(3);
/// assert_eq!(single.to_string(), "3");
///
/// let range = PageRangeChunk::StartEnd(2, 5);
/// assert_eq!(range.to_string(), "2-5");
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PageRangeChunk {
    /// A single page number.
    SingleValue(usize),

    /// A range of pages, from `start` to `end` inclusive.
    StartEnd(usize, usize),
}

impl PageRangeChunk {
    /// Checks if the given number is within the range represented by this `PageRangeChunk`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gotenberg_pdf::PageRangeChunk;
    ///
    /// let single = PageRangeChunk::SingleValue(3);
    /// assert!(single.in_range(3));
    /// assert!(!single.in_range(4));
    ///
    /// let range = PageRangeChunk::StartEnd(2, 5);
    /// assert!(range.in_range(2));
    /// assert!(range.in_range(3));
    /// assert!(range.in_range(4));
    /// assert!(range.in_range(5));
    /// assert!(!range.in_range(6));
    /// ```
    pub fn in_range(&self, number: usize) -> bool {
        match self {
            PageRangeChunk::SingleValue(value) => *value == number,
            PageRangeChunk::StartEnd(start, end) => *start <= number && number <= *end,
        }
    }
}

impl FromStr for PageRangeChunk {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_trimmed = s.trim(); // Trim whitespace
        if let Some((start, end)) = s_trimmed.split_once('-') {
            let start = start.trim().parse::<usize>().map_err(|_| {
                Error::ParseError(
                    "PageRangeChunk".to_string(),
                    s.to_string(),
                    format!("Invalid integer: {}", start),
                )
            })?;
            let end = end.trim().parse::<usize>().map_err(|_| {
                Error::ParseError(
                    "PageRangeChunk".to_string(),
                    s.to_string(),
                    format!("Invalid integer: {}", start),
                )
            })?;
            if start > end {
                Err(Error::ParseError(
                    "PageRangeChunk".to_string(),
                    s.to_string(),
                    "Start cannot be greater than end".to_string(),
                ))
            } else {
                Ok(PageRangeChunk::StartEnd(start, end))
            }
        } else {
            let value = s.parse::<usize>().map_err(|_| {
                Error::ParseError(
                    "PageRangeChunk".to_string(),
                    s.to_string(),
                    format!("Invalid integer: {}", s),
                )
            })?;
            Ok(PageRangeChunk::SingleValue(value))
        }
    }
}

impl Serialize for PageRangeChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PageRangeChunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PageRangeChunk::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for PageRangeChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageRangeChunk::SingleValue(value) => write!(f, "{}", value),
            PageRangeChunk::StartEnd(start, end) => write!(f, "{}-{}", start, end),
        }
    }
}

impl Serialize for PageRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = self.to_string();
        serializer.serialize_str(&formatted)
    }
}

impl<'de> Deserialize<'de> for PageRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PageRange::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        "1".parse::<PageRangeChunk>().unwrap();
        "2-5".parse::<PageRangeChunk>().unwrap();
        assert!("10-7".parse::<PageRangeChunk>().is_err());
        assert!("abc".parse::<PageRangeChunk>().is_err());
    }

    #[test]
    fn test_in_range() {
        let single = PageRangeChunk::SingleValue(3);
        assert!(single.in_range(3));
        assert!(!single.in_range(2));
        assert!(!single.in_range(4));

        let range = PageRangeChunk::StartEnd(2, 5);
        assert!(range.in_range(2));
        assert!(range.in_range(3));
        assert!(range.in_range(5));
        assert!(!range.in_range(1));
        assert!(!range.in_range(6));
    }

    #[test]
    fn test_to_string() {
        assert_eq!(PageRangeChunk::SingleValue(3).to_string(), "3");
        assert_eq!(PageRangeChunk::StartEnd(1, 4).to_string(), "1-4");
    }

    #[test]
    fn test_page_range_from_str() {
        let range: PageRange = "1, 3-5, 7".parse().unwrap();
        assert_eq!(
            range.chunks,
            vec![
                PageRangeChunk::SingleValue(1),
                PageRangeChunk::StartEnd(3, 5),
                PageRangeChunk::SingleValue(7)
            ]
        );

        let invalid: Result<PageRange, _> = "1,abc,5-".parse();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_page_range_to_string() {
        let range = PageRange {
            chunks: vec![
                PageRangeChunk::SingleValue(1),
                PageRangeChunk::StartEnd(3, 5),
                PageRangeChunk::SingleValue(7),
            ],
        };
        assert_eq!(range.to_string(), "1,3-5,7");
    }

    #[test]
    fn test_page_range_in_range() {
        let range: PageRange = "1,3-5,7".parse().unwrap();
        assert!(range.in_range(3)); // Matches PageRangeChunk::StartEnd(3, 5)
        assert!(range.in_range(7)); // Matches PageRangeChunk::SingleValue(7)
        assert!(!range.in_range(6)); // No match
        assert!(!range.in_range(0)); // Out of bounds
    }

    #[test]
    fn test_empty_page_range() {
        let range: PageRange = "".parse().unwrap();
        assert!(range.in_range(0)); // Empty range includes all pages
        assert!(range.in_range(1));
        assert!(range.in_range(2));

        assert_eq!(range.to_string(), "");
    }
}
