use std::{error::Error, str::FromStr};

pub fn try_func<T, R, F>(mut f: F, t: Option<T>) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> R,
{
    if let Some(mut t) = t {
        f(&mut t);
    } else {
        loop_func(f)?;
    }

    Ok(())
}

pub fn loop_func<T, R, F>(mut f: F) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> R,
{
    for line in std::io::stdin().lines() {
        let mut t = T::from_str(&line?)?;
        f(&mut t);
    }

    Ok(())
}

pub fn try_func_once<T, R, F>(mut f: F, t: Option<T>) -> Result<(), Box<dyn Error>>
where
    T: FromStr + 'static,
    <T as FromStr>::Err: Error,
    F: FnMut(&mut T) -> R,
{
    if let Some(mut t) = t {
        f(&mut t);
    } else {
        let line = std::io::stdin().lines().next().unwrap()?;
        let mut t = T::from_str(&line)?;
        f(&mut t);
    }

    Ok(())
}
