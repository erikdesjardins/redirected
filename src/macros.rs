#[cfg(test)]
macro_rules! case {
    ( $name:ident : $body:expr ) => {
        #[test]
        fn $name() {
            $body
        }
    };
}
