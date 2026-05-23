use anyhowed::{Error, Result, Context, anyhow, bail, ensure};

#[test]
fn test_error_creation() {
    let err = Error::msg("simple error");
    assert_eq!(err.to_string(), "simple error");
    
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = Error::new(io_err);
    assert!(err.to_string().contains("file not found"));
}

#[test]
fn test_context_chain() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err = Error::new(io_err)
        .context("reading config file")
        .context("initializing application");
    
    let err_str = err.to_string();
    assert_eq!(err_str, "initializing application");
    
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("initializing application"));
    assert!(debug_str.contains("reading config file"));
    assert!(debug_str.contains("file missing"));
}

#[test]
fn test_root_cause() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err = Error::new(io_err)
        .context("reading config")
        .context("initializing");
    
    let root = err.root_cause();
    assert!(root.to_string().contains("file missing"));
}

#[test]
fn test_chain_iterator() {
    let err = Error::msg("top level")
        .context("middle")
        .context("bottom");
    
    let chain: Vec<_> = err.chain().collect();
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].to_string(), "bottom");
    assert_eq!(chain[1].to_string(), "middle");
    assert_eq!(chain[2].to_string(), "top level");
}

#[test]
fn test_downcast() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let err = Error::new(io_err);
    
    assert!(err.is::<std::io::Error>());
    assert!(!err.is::<std::fmt::Error>());
    
    let downcast_ref = err.downcast_ref::<std::io::Error>();
    assert!(downcast_ref.is_some());
    assert_eq!(downcast_ref.unwrap().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_from_trait() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "io error");
    let err: Error = io_err.into();
    assert!(err.is::<std::io::Error>());
}

#[test]
fn test_result_context() -> Result<()> {
    let result: Result<i32> = Err(Error::msg("original error"));
    let result = result.context("adding context");
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "adding context");
    
    let chain: Vec<_> = err.chain().collect();
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[1].to_string(), "original error");
    
    Ok(())
}

#[test]
fn test_result_with_context() -> Result<()> {
    let result: Result<i32> = Err(Error::msg("base error"));
    let result = result.with_context(|| format!("context from closure: {}", 42));
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "context from closure: 42");
    
    Ok(())
}

#[test]
fn test_std_result_context() -> anyhowed::Result<()> {
    let io_result: std::result::Result<(), std::io::Error> = 
        Err(std::io::Error::new(std::io::ErrorKind::Other, "io error"));
    
    let result: anyhowed::Result<()> = io_result.context("failed operation");
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "failed operation");
    
    Ok(())
}

#[test]
fn test_option_context() -> Result<()> {
    let opt: Option<i32> = None;
    let result = opt.context("value was missing");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "value was missing");
    
    let opt: Option<i32> = Some(42);
    let result = opt.context("value was missing");
    assert_eq!(result.unwrap(), 42);
    
    Ok(())
}

#[test]
fn test_option_with_context() -> Result<()> {
    let opt: Option<i32> = None;
    let result = opt.with_context(|| format!("context: {}", "dynamic"));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "context: dynamic");
    
    Ok(())
}

#[test]
fn test_anyhow_macro() {
    let err = anyhow!("simple error message");
    assert_eq!(err.to_string(), "simple error message");
    
    let err = anyhow!("error with {}: {}", "value", 42);
    assert_eq!(err.to_string(), "error with value: 42");
    
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let err = anyhow!(io_err);
    assert!(err.is::<std::io::Error>());
}

#[test]
fn test_bail_macro() -> Result<()> {
    fn test_fn() -> Result<()> {
        bail!("something went wrong");
    }
    
    let result = test_fn();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "something went wrong");
    
    fn test_fn2() -> Result<()> {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "io error");
        bail!(io_err);
    }
    
    let result = test_fn2();
    assert!(result.is_err());
    assert!(result.unwrap_err().is::<std::io::Error>());
    
    Ok(())
}

#[test]
fn test_ensure_macro() -> Result<()> {
    fn test_fn(cond: bool) -> Result<()> {
        ensure!(cond, "condition failed");
        Ok(())
    }
    
    assert!(test_fn(true).is_ok());
    let err = test_fn(false).unwrap_err();
    assert_eq!(err.to_string(), "condition failed");
    
    fn test_fn2(cond: bool) -> Result<()> {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "custom error");
        ensure!(cond, err);
        Ok(())
    }
    
    assert!(test_fn2(true).is_ok());
    let err = test_fn2(false).unwrap_err();
    assert!(err.is::<std::io::Error>());
    
    Ok(())
}

#[test]
fn test_complex_error_chain() -> Result<()> {
    fn read_config() -> std::result::Result<String, std::io::Error> {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "config.toml"))
    }
    
    fn load_config() -> Result<String> {
        let config = read_config()
            .context("failed to read config file")?;
        Ok(config)
    }
    
    fn init_app() -> Result<()> {
        let _config = load_config()
            .context("failed to initialize application")?;
        Ok(())
    }
    
    let result = init_app();
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("failed to initialize application"));
    assert!(debug_str.contains("failed to read config file"));
    assert!(debug_str.contains("config.toml"));
    
    Ok(())
}

#[test]
fn test_nested_context() {
    let err = Error::msg("original")
        .context("level 1")
        .context("level 2")
        .context("level 3");
    
    let chain: Vec<_> = err.chain().collect();
    assert_eq!(chain.len(), 4);
    assert_eq!(chain[0].to_string(), "level 3");
    assert_eq!(chain[1].to_string(), "level 2");
    assert_eq!(chain[2].to_string(), "level 1");
    assert_eq!(chain[3].to_string(), "original");
}

#[test]
fn test_error_conversion_to_boxed() {
    let err = Error::msg("test error");
    let boxed: Box<dyn std::error::Error> = err.into();
    assert_eq!(boxed.to_string(), "test error");
    
    let err = Error::msg("test error");
    let boxed_send_sync: Box<dyn std::error::Error + Send + Sync> = err.into();
    assert_eq!(boxed_send_sync.to_string(), "test error");
}

#[test]
fn test_message_error_impl() {
    let err = Error::msg("custom message");
    assert_eq!(err.to_string(), "custom message");
    
    let boxed: Box<dyn std::error::Error> = err.into();
    assert_eq!(boxed.to_string(), "custom message");
}

#[test]
fn test_context_error_impl() {
    let base_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err = Error::new(base_err).context("context message");
    
    assert_eq!(err.to_string(), "context message");
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("context message"));
    assert!(debug_str.contains("file missing"));
}