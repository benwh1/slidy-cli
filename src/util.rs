use std::{error::Error, str::FromStr};

pub fn try_fn<T, R, F>(mut f: F, t: Option<T>) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> R,
{
    try_fallible_fn(|a| Ok(f(a)), t)
}

pub fn try_fallible_fn<T, R, F>(mut f: F, t: Option<T>) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> Result<R, Box<dyn Error>>,
{
    if let Some(mut t) = t {
        f(&mut t)?;
    } else {
        loop_fn(f)?;
    }

    Ok(())
}

pub fn loop_fn<T, R, F>(mut f: F) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> Result<R, Box<dyn Error>>,
{
    for (i, line) in std::io::stdin().lines().enumerate() {
        let line_number = i + 1;
        let mut t = parse(&line?).map_err(|e| format!("line {line_number}: {e}"))?;
        f(&mut t).map_err(|e| format!("line {line_number}: {e}"))?;
    }

    Ok(())
}

pub fn try_fallible_fn_once<T, R, F>(mut f: F, t: Option<T>) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> Result<R, Box<dyn Error>>,
{
    if let Some(mut t) = t {
        f(&mut t)?;
    } else {
        let line = std::io::stdin()
            .lines()
            .next()
            .ok_or("no input given on stdin")??;
        let mut t = parse(&line)?;
        f(&mut t)?;
    }

    Ok(())
}

/// Parses one line of input, with the input included in the error message.
fn parse<T>(s: &str) -> Result<T, Box<dyn Error>>
where
    T: FromStr,
    <T as FromStr>::Err: Error,
{
    Ok(T::from_str(s).map_err(|e| format!("could not parse `{}`: {e}", s.trim()))?)
}
