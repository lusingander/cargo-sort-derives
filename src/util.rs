pub fn parse_order(order: String) -> Vec<String> {
    order
        .split(',')
        .map(str::trim)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_order() {
        let order = "A, B, C".to_string();
        let expected: Vec<String> = vec!["A", "B", "C"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let actual = parse_order(order);

        assert_eq!(actual, expected);
    }
}
