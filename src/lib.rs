/// DDR song representation and searching
pub mod ddr_song;
/// The backend logic for querying and parsing of DDR score websites
pub mod score_websites;
/// Structures and methods related to storing the scores of players
pub mod scores;

/// `reqwest`'s async http client re-exported
pub use reqwest::Client;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
