use std::{
    fmt::{Display, Write},
    str::FromStr,
};

use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct Size(pub u32, pub u32);

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())?;
        f.write_char('x')?;
        f.write_str(&self.1.to_string())
    }
}

#[derive(Debug, Error)]
#[error("ParseSizeError")]
pub struct ParseSizeError;

impl FromStr for Size {
    type Err = ParseSizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(s) = s.parse::<u32>() {
            Ok(Size(s, s))
        } else {
            let (w, h) = s.split_once('x').ok_or(ParseSizeError)?;
            let (w, h) = (
                w.trim().parse::<u32>().map_err(|_| ParseSizeError)?,
                h.trim().parse::<u32>().map_err(|_| ParseSizeError)?,
            );
            Ok(Size(w, h))
        }
    }
}
