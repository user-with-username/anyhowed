extern crate anyhowed;

use anyhowed::{Error, Result, Context, anyhow, bail, ensure};
use std::fmt;
use std::error::Error as StdError;

#[derive(Debug)]
struct CustomError {
    msg: String,
}

impl CustomError {
    fn new(msg: &str) -> Self {
        CustomError { msg: msg.to_string() }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CustomError: {}", self.msg)
    }
}

impl StdError for CustomError {}

fn produce_result() -> Result<(), CustomError> {
    Err(CustomError::new("original"))
}

fn produce_option() -> Option<()> {
    None
}

#[test]
fn test_error_new() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "io failure");
    let err = Error::new(io_err);
    assert_eq!(err.to_string(), "io failure");
    assert!(err.source().is_none());
    assert!(err.backtrace().is_some());
}

#[test]
fn test_error_msg() {
    let err = Error::msg("something went wrong");
    assert_eq!(err.to_string(), "something went wrong");
    assert!(err.source().is_none());
}

#[test]
fn test_context() {
    let base_err = Error::msg("base error");
    let err = base_err.context("additional context");
    assert_eq!(err.to_string(), "additional context");
    let mut chain = err.chain();
    assert_eq!(chain.next().unwrap().to_string(), "additional context");
    assert_eq!(chain.next().unwrap().to_string(), "base error");
    assert!(chain.next().is_none());
}

#[test]
fn test_chain_and_root_cause() {
    let err = Error::msg("root").context("middle").context("top");
    let chain: Vec<_> = err.chain().collect();
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].to_string(), "top");
    assert_eq!(chain[1].to_string(), "middle");
    assert_eq!(chain[2].to_string(), "root");
    assert_eq!(err.root_cause().to_string(), "root");
}

#[test]
fn test_source() {
    let err = Error::msg("leaf").context("context");
    assert_eq!(err.source().unwrap().to_string(), "leaf");
    let err2 = Error::msg("only one");
    assert!(err2.source().is_none());
}

#[test]
fn test_downcast_ref() {
    let custom = CustomError::new("test");
    let err = Error::new(custom);
    let downcasted: Option<&CustomError> = err.downcast_ref::<CustomError>();
    assert!(downcasted.is_some());
    assert_eq!(downcasted.unwrap().to_string(), "CustomError: test");

    let downcasted_io: Option<&std::io::Error> = err.downcast_ref::<std::io::Error>();
    assert!(downcasted_io.is_none());
}

#[test]
fn test_downcast_mut() {
    let custom = CustomError::new("test");
    let mut err = Error::new(custom);
    if let Some(c) = err.downcast_mut::<CustomError>() {
        c.msg = "modified".to_string();
    }
    let downcasted: Option<&CustomError> = err.downcast_ref::<CustomError>();
    assert_eq!(downcasted.unwrap().to_string(), "CustomError: modified");
}

#[test]
fn test_downcast() {
    let err = Error::msg("plain");
    match err.downcast::<CustomError>() {
        Err(returned_err) => {
            assert_eq!(returned_err.to_string(), "plain");
        }
        Ok(_) => panic!("downcast should have failed"),
    }
}

#[test]
fn test_is() {
    let err = Error::new(CustomError::new("boom"));
    assert!(err.is::<CustomError>());
    assert!(!err.is::<std::io::Error>());
}

#[test]
fn test_display() {
    let err = Error::msg("display test").context("ctx");
    let display_str = format!("{}", err);
    assert_eq!(display_str, "ctx");
}

#[test]
fn test_debug_does_not_panic() {
    let err = Error::msg("debug test").context("ctx");
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("error:"));
    assert!(debug_str.contains("ctx"));
    assert!(debug_str.contains("Caused by:"));
    assert!(debug_str.contains("debug test"));
}

#[test]
fn test_context_for_result() {
    let res: Result<(), CustomError> = produce_result();
    let err_res = res.context("failed to produce");
    assert!(err_res.is_err());
    let err = err_res.unwrap_err();
    assert_eq!(err.to_string(), "failed to produce");
    let mut chain = err.chain();
    assert_eq!(chain.next().unwrap().to_string(), "failed to produce");
    assert_eq!(chain.next().unwrap().to_string(), "CustomError: original");
}

#[test]
fn test_with_context_for_result() {
    let res: Result<(), CustomError> = produce_result();
    let err_res = res.with_context(|| format!("context from closure: {}", 42));
    assert!(err_res.is_err());
    let err = err_res.unwrap_err();
    assert_eq!(err.to_string(), "context from closure: 42");
}

#[test]
fn test_context_for_option() {
    let opt = produce_option();
    let res = opt.context("option was None");
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.to_string(), "option was None");
    assert!(err.source().is_none());
}

#[test]
fn test_with_context_for_option() {
    let opt = produce_option();
    let res = opt.with_context(|| "computed context".to_string());
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.to_string(), "computed context");
}

#[test]
fn test_anyhow_macro() {
    let err = anyhow!("literal error");
    assert_eq!(err.to_string(), "literal error");

    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "io");
    let err = anyhow!(io_err);
    assert_eq!(err.to_string(), "io");

    let code = 404;
    let err = anyhow!("http status: {}", code);
    assert_eq!(err.to_string(), "http status: 404");
}

#[test]
fn test_bail_macro() {
    fn check_bail(flag: bool) -> Result<()> {
        if flag {
            bail!("bail with literal");
        }
        Ok(())
    }
    let err = check_bail(true).unwrap_err();
    assert_eq!(err.to_string(), "bail with literal");

    fn check_bail_err() -> Result<()> {
        let e = CustomError::new("custom");
        bail!(e);
    }
    let err = check_bail_err().unwrap_err();
    assert_eq!(err.to_string(), "CustomError: custom");

    fn check_bail_fmt() -> Result<()> {
        let x = 99;
        bail!("value is {}", x);
    }
    let err = check_bail_fmt().unwrap_err();
    assert_eq!(err.to_string(), "value is 99");
}

#[test]
fn test_ensure_macro() {
    fn check_ensure(cond: bool) -> Result<()> {
        ensure!(cond, "condition failed");
        Ok(())
    }
    let err = check_ensure(false).unwrap_err();
    assert_eq!(err.to_string(), "condition failed");

    fn check_ensure_with_err() -> Result<()> {
        let e = CustomError::new("custom condition");
        ensure!(false, e);
        Ok(())
    }
    let err = check_ensure_with_err().unwrap_err();
    assert_eq!(err.to_string(), "CustomError: custom condition");

    fn check_ensure_fmt() -> Result<()> {
        let val = 0;
        ensure!(val > 0, "value must be positive, got {}", val);
        Ok(())
    }
    let err = check_ensure_fmt().unwrap_err();
    assert_eq!(err.to_string(), "value must be positive, got 0");
}