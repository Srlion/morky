macro_rules! regex {
    ($re:expr $(,)?) => {{
        static RE: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new($re).expect("invalid regex"));
        &*RE
    }};
}

pub use regex;
