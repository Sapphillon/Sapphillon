use anyhow::{Result, Error};

fn fetch(url: &str) -> Result<String> {
    let response = reqwest::blocking::get(url)?;
    if response.status().is_success() {
        let body = response.text()?;
        Ok(body)
    } else {
        Err(Error::msg(format!("Failed to fetch URL: {url}")))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch() {
        let url = "https://dummyjson.com/test";
        let result = fetch(url);
        assert!(result.is_ok());
        let body = result.unwrap();
        assert!(body.contains("ok"));
        println!("Fetched content: {body}");
        
    }
}
