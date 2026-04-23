pub fn is_fqdn(domain: &str) -> bool {
    let domain = domain.trim_end_matches('.'); // strip optional trailing dot
    let parts: Vec<&str> = domain.split('.').collect();
    parts.len() >= 2                          // at least domain + TLD
    && parts.iter().all(|p| !p.is_empty())    // no empty labels
    && domain.len() <= 253                    // DNS max length
    && parts.iter().all(|p| p.len() <= 63)   // label max length
    && parts.iter().all(|p| {                 // valid chars only
        p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !p.starts_with('-')
        && !p.ends_with('-')
    })
    && parts.last().map_or(false, |tld| tld.chars().any(|c| c.is_ascii_alphabetic()))
}
