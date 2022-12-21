use std::{error::Error, str::FromStr};

pub fn try_func<T: FromStr + 'static, R, F: Fn(&mut T) -> R>(
    f: F,
    t: Option<T>,
) -> Result<(), Box<dyn Error>>
where
    <T as FromStr>::Err: Error,
{
    if let Some(mut t) = t {
        f(&mut t);
    } else {
        for line in std::io::stdin().lines() {
            let mut t = T::from_str(&line?)?;
            f(&mut t);
        }
    }

    Ok(())
}

pub fn try_func_once<T: FromStr + 'static, R, F: Fn(&mut T) -> R>(
    f: F,
    t: Option<T>,
) -> Result<(), Box<dyn Error>>
where
    <T as FromStr>::Err: Error,
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
