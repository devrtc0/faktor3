use std::{
    env, error,
    fs::File,
    io::{BufRead, BufReader},
};

pub trait SetEnvVar {
    fn set(&self, key: &str, value: Option<&str>) -> Result<(), Box<dyn error::Error>>;
}

#[derive(Debug, Clone, Copy)]
pub struct Override;

#[derive(Debug, Clone, Copy)]
pub struct Skip;

impl SetEnvVar for Override {
    fn set(&self, key: &str, value: Option<&str>) -> Result<(), Box<dyn error::Error>> {
        match value {
            Some("") | None => env::remove_var(key),
            Some(val) => env::set_var(key, val),
        }
        Ok(())
    }
}

impl SetEnvVar for Skip {
    fn set(&self, key: &str, value: Option<&str>) -> Result<(), Box<dyn error::Error>> {
        if let Some(value) = value {
            match env::var(key) {
                Err(env::VarError::NotPresent) => env::set_var(key, value),
                _ => {}
            }
        }
        Ok(())
    }
}

pub fn init<T>(mode: T) -> Result<(), Box<dyn error::Error>>
where
    T: SetEnvVar,
{
    from_file(".env", mode)
}

pub fn from_file<T>(filename: &str, mode: T) -> Result<(), Box<dyn error::Error>>
where
    T: SetEnvVar,
{
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    init_inner(reader, mode)
}

fn init_inner<R, T>(reader: R, mode: T) -> Result<(), Box<dyn error::Error>>
where
    T: SetEnvVar,
    R: BufRead,
{
    let lines = reader.lines();

    for line in lines {
        let line = line?;
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let (key, value) = split_once(line);
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        mode.set(key, value).unwrap();
    }
    Ok(())
}

fn split_once(in_string: &str) -> (&str, Option<&str>) {
    let mut splitter = in_string.splitn(2, '=');
    let first = splitter.next().unwrap();
    let second = splitter.next();
    (first, second)
}

#[cfg(test)]
mod tests {
    use env::VarError;

    use crate::*;

    #[test]
    fn test_split_once_empty_string() {
        let s = "";
        let (f, s) = split_once(s);
        assert_eq!("", f);
        assert_eq!(None, s);
    }

    #[test]
    fn test_split_once_no_equal_sign() {
        let s = "TEST";
        let (f, s) = split_once(s);
        assert_eq!("TEST", f);
        assert_eq!(None, s);
    }

    #[test]
    fn test_split_once_no_value() {
        let s = "TEST=";
        let (f, s) = split_once(s);
        assert_eq!("TEST", f);
        assert_eq!(Some(""), s);
    }

    #[test]
    fn test_split_once_with_value() {
        let s = "TEST=value";
        let (f, s) = split_once(s);
        assert_eq!("TEST", f);
        assert_eq!(Some("value"), s);
    }

    #[test]
    fn test_split_once_with_quotes() {
        let s = r#"TEST="value""#;
        let (f, s) = split_once(s);
        assert_eq!("TEST", f);
        assert_eq!(Some(r#""value""#), s);
    }

    #[test]
    fn test_split_once_with_multiple_equals() {
        let s = "TEST=value=1=2=3=";
        let (f, s) = split_once(s);
        assert_eq!("TEST", f);
        assert_eq!(Some("value=1=2=3="), s);
    }

    #[test]
    fn test_init_override_set_when_no_value() {
        env::remove_var("TESTVALUE");
        assert_eq!(Err(VarError::NotPresent), env::var("TESTKEY"));

        let a = "TESTKEY=TESTVALUE".as_bytes();
        let res = init_inner(a, crate::Override);
        assert_eq!(true, res.is_ok());
        assert_eq!("TESTVALUE", env::var("TESTKEY").unwrap());
    }

    #[test]
    fn test_init_override_value() {
        env::remove_var("TESTVALUE");
        env::set_var("TESTKEY", "OLDTESTVALUE");

        let a = "TESTKEY=TESTVALUE".as_bytes();
        let res = init_inner(a, crate::Override);
        assert_eq!(true, res.is_ok());
        assert_eq!("TESTVALUE", env::var("TESTKEY").unwrap());
    }

    #[test]
    fn test_init_skip_value() {
        env::remove_var("TESTVALUE");
        env::set_var("TESTKEY", "OLDTESTVALUE");

        let a = "TESTKEY=TESTVALUE".as_bytes();
        let res = init_inner(a, crate::Skip);
        assert_eq!(true, res.is_ok());
        assert_eq!("OLDTESTVALUE", env::var("TESTKEY").unwrap());
    }
}
