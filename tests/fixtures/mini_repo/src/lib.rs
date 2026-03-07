pub mod nested;

pub struct Data {
    pub value: usize,
}

impl Data {
    pub fn incremented(&self) -> usize {
        nested::helper(self.value)
    }
}

pub fn example_sum(left: usize, right: usize) -> usize {
    left + right
}

pub fn oversized_example(limit: usize) -> usize {
    let mut total = 0;
    for value in 0..limit {
        if value % 2 == 0 {
            total += value;
        } else {
            total += value * 2;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::{Data, example_sum};

    #[test]
    fn example_sum_smoke() {
        assert_eq!(example_sum(2, 3), 5);
    }

    #[test]
    fn data_incremented_smoke() {
        let data = Data { value: 4 };
        assert_eq!(data.incremented(), 5);
    }
}
