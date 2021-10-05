pub mod ddr_song;
pub mod sanbai;
pub mod scores;
pub mod skill_attack;

pub use reqwest::Client;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
