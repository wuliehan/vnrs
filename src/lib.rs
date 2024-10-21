pub fn add(left: usize, right: usize) -> usize {
    left + right
}
pub mod vnrs;
pub mod vnrs_ctastrategy;
// pub use vnrs_ctastrategy::backtesting::BacktestingEngine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn works(){
        // let bte=BacktestingEngine{};
    }
}
