pub mod ddr_song;
pub mod score_websites;
pub mod scores;

pub use reqwest::Client;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
