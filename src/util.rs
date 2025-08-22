use reqwest::Url;

#[cfg(windows)]
pub fn open_link(url: &Url) {
    use std::{os::windows::process::CommandExt, process::Command};

    let _ = Command::new("cmd")
        .args(["/C", "start", url.as_str()])
        .creation_flags(0x08000000) // Don't create a window
        .status();
}

#[cfg(not(windows))]
pub fn open_link(url: &Url) {
    let _ = Command::new("xdg-open")
        .arg(url.as_str())
        .status();
}

/// Builds a Url query string from the given [params].
/// The parameter and their values won't be escaped!
/// This method is supposed to be used with [Url::set_query].
/// As such, the leading '?' is not included.
pub fn build_query_string<Q,V>(params: Q) -> String 
where 
    V: AsRef<str>,
    Q: IntoIterator<Item=(V,V)>
{
    let mut query = String::new();
    for (k,v) in params.into_iter() {
        query.push_str(k.as_ref());
        query.push('=');
        query.push_str(v.as_ref());
        query.push('&');
    }
    let _ = query.pop(); // Remove trailing '&'
    query
}
