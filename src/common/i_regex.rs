#[macro_export]
macro_rules! regex {
    ($re:expr $(,)?) => {{
        static RE: std::sync::LazyLock<regex_lite::Regex> =
            std::sync::LazyLock::new(|| regex_lite::Regex::new($re).expect("invalid regex"));
        &*RE
    }};
}

pub use regex;
