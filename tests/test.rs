macro_rules! foo {
    ($t:tt) => {
        foo! { ($t) ($t) }
    };
    ($t:tt (0)) => {
        set_span::set_span! {$t[0], {
            #set_span {
                compile_error! {"input by zero"}
            }
        }}
    };
    ($_:tt ($lit:literal)) => {
        // ...
    };
}

fn main() {
    set_span::set_span!(x, {
        let a = "".to_owned();
    });
    set_span::set_span!([(((x)))][0][0][0], {
        #set_span(
            dbg!(a);
        )
    });
    foo!(1);
}
