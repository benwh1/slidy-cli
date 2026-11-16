use std::{error::Error, io::IsTerminal as _, str::FromStr};

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
        let mut t = parse(&read_single_line()?)?;
        f(&mut t)?;
    }

    Ok(())
}

/// Reads one line from stdin, and checks that there are no more lines after it.
fn read_single_line() -> Result<String, Box<dyn Error>> {
    let mut lines = std::io::stdin().lines();
    let line = lines.next().ok_or("no input given on stdin")??;

    // Don't silently ignore extra input. Only check when stdin is piped: on a terminal, this would
    // wait for the user to press Ctrl-D.
    let extra = if std::io::stdin().is_terminal() {
        None
    } else {
        lines.find(|l| l.as_ref().map_or(true, |l| !l.trim().is_empty()))
    };
    drop(lines);

    if extra.transpose()?.is_some() {
        Err("expected a single input on stdin, but got more than one line")?;
    }

    Ok(line)
}

/// Parses one line of input, with the input included in the error message.
fn parse<T>(s: &str) -> Result<T, Box<dyn Error>>
where
    T: FromStr,
    <T as FromStr>::Err: Error,
{
    Ok(T::from_str(s).map_err(|e| format!("could not parse `{}`: {e}", s.trim()))?)
}
