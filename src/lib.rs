mod cachepadded;
mod ebr;
mod engine;
mod file;
mod io;
mod lazy;
mod lftt;
mod mdlist;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
