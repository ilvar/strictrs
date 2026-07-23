pub fn first(values: &[u32]) -> Option<&u32> {
    values.first()
}

#[cfg(test)]
mod tests {
    use super::first;

    #[test]
    fn panic_api_is_allowed_in_tests() {
        let values = [1];
        assert_eq!(*first(&values).unwrap(), 1);
    }
}
