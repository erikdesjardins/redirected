#[cfg(test)]
macro_rules! case {
    ( $name:ident : $body:expr ) => {
        #[test]
        fn $name() {
            $body
        }
    };
}

#[cfg(test)]
macro_rules! assert_matches {
    ( $expected:pat, $input:expr ) => {{
        match $input {
            $expected => {}
            not_expected => assert!(
                false,
                "{:?} does not match {}",
                not_expected,
                stringify!($expected)
            ),
        }
    }};
}
