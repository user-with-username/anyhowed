use std::fmt;
use std::io::{stderr, IsTerminal};
use std::backtrace::Backtrace;

struct ErrorAsStdError(Error);

impl fmt::Debug for ErrorAsStdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl fmt::Display for ErrorAsStdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl std::error::Error for ErrorAsStdError {}

pub struct Error {
    chain: Vec<Box<dyn std::error::Error + Send + Sync + 'static>>,
    backtrace: Option<Backtrace>,
}

impl Error {
    pub fn new<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Error {
            chain: vec![Box::new(error)],
            backtrace: Some(Backtrace::capture()),
        }
    }

    #[track_caller]
    pub fn msg<M>(message: M) -> Self
    where
        M: fmt::Display + Send + Sync + 'static,
    {
        Error {
            chain: vec![Box::new(MessageError(message.to_string()))],
            backtrace: Some(Backtrace::capture()),
        }
    }

    pub fn context<C>(mut self, context: C) -> Self
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.chain.insert(0, Box::new(ContextError(context.to_string())));
        self
    }

    pub fn chain(&self) -> impl Iterator<Item = &dyn std::error::Error> {
        self.chain.iter().map(|e| e.as_ref() as &dyn std::error::Error)
    }

    pub fn root_cause(&self) -> &dyn std::error::Error {
        self.chain.last().map(|e| e.as_ref()).unwrap()
    }

    pub fn backtrace(&self) -> Option<&Backtrace> {
        self.backtrace.as_ref()
    }

    pub fn downcast<T: std::error::Error + 'static>(self) -> Result<T, Self> {
        Err(self)
    }

    pub fn downcast_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        self.chain.first().and_then(|e| {
            (e.as_ref() as &dyn std::error::Error).downcast_ref::<T>()
        })
    }

    pub fn downcast_mut<T: std::error::Error + 'static>(&mut self) -> Option<&mut T> {
        self.chain.first_mut().and_then(|e| {
            (e.as_mut() as &mut dyn std::error::Error).downcast_mut::<T>()
        })
    }

    pub fn is<T: std::error::Error + 'static>(&self) -> bool {
        self.downcast_ref::<T>().is_some()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(first) = self.chain.first() {
            write!(f, "{}", first)
        } else {
            write!(f, "unknown error")
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let use_colors = stderr().is_terminal();

        if use_colors {
            write!(f, "\r\x1B[K\x1b[91merror:\x1b[0m ")?;
        } else {
            write!(f, "error: ")?;
        }

        if let Some(first) = self.chain.first() {
            write!(f, "{}", first)?;
        }

        let mut causes = self.chain.iter().skip(1).peekable();
        if causes.peek().is_some() {
            writeln!(f)?;
            writeln!(f, "\nCaused by:")?;
            for cause in causes {
                writeln!(f, "    {}", cause)?;
            }
        }

        if let Some(bt) = &self.backtrace {
            let bt_str = format!("{}", bt);
            if !bt_str.contains("disabled") && !bt_str.is_empty() {
                writeln!(f, "\nStack backtrace:")?;
                writeln!(f, "{}", bt)?;
            }
        }

        Ok(())
    }
}

impl From<Error> for Box<dyn std::error::Error> {
    fn from(e: Error) -> Self {
        Box::new(ErrorAsStdError(e))
    }
}

impl From<Error> for Box<dyn std::error::Error + Send + Sync> {
    fn from(e: Error) -> Self {
        Box::new(ErrorAsStdError(e))
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Error::new(error)
    }
}

#[derive(Debug)]
struct MessageError(String);

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MessageError {}

#[derive(Debug)]
struct ContextError(String);

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ContextError {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait Context<T, E> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static;

    fn with_context<C, F>(self, context: F) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| Error::new(e).context(context))
    }

    fn with_context<C, F>(self, context: F) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| Error::new(e).context(context()))
    }
}

impl<T> Context<T, Error> for std::result::Result<T, Error> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| e.context(context))
    }

    fn with_context<C, F>(self, context: F) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.context(context()))
    }
}

impl<T> Context<T, std::convert::Infallible> for Option<T> {
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| Error::msg(context))
    }

    fn with_context<C, F>(self, context: F) -> Result<T, Error>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Error::msg(context()))
    }
}

#[macro_export]
macro_rules! anyhow {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::Error::msg(format!($fmt $(, $arg)*))
    };
    ($err:expr $(,)?) => {
        $crate::Error::new($err)
    };
}

#[macro_export]
macro_rules! bail {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        return $crate::Result::Err($crate::Error::msg(format!($fmt $(, $arg)*)))
    };
    ($err:expr $(,)?) => {
        return $crate::Result::Err($crate::Error::new($err))
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        if !$cond {
            return $crate::Result::Err($crate::Error::msg(format!($fmt $(, $arg)*)));
        }
    };
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return $crate::Result::Err($crate::Error::new($err));
        }
    };
}