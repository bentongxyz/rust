use super::{check, check_no_mismatches, check_types};

#[test]
fn block_expr_type_mismatch() {
    // FIXME fix double type mismatch
    check(
        r"
fn test() {
    let a: i32 = { 1i64 };
              // ^^^^^^^^ expected i32, got i64
                // ^^^^ expected i32, got i64
}
        ",
    );
}

#[test]
fn coerce_places() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
struct S<T> { a: T }

fn f<T>(_: &[T]) -> T { loop {} }
fn g<T>(_: S<&[T]>) -> T { loop {} }

fn gen<T>() -> *mut [T; 2] { loop {} }
fn test1<U>() -> *mut [U] {
    gen()
}

fn test2() {
    let arr: &[u8; 1] = &[1];

    let a: &[_] = arr;
    let b = f(arr);
    let c: &[_] = { arr };
    let d = g(S { a: arr });
    let e: [&[_]; 1] = [arr];
    let f: [&[_]; 2] = [arr; 2];
    let g: (&[_], &[_]) = (arr, arr);
}
"#,
    );
}

#[test]
fn let_stmt_coerce() {
    check(
        r"
//- minicore: coerce_unsized
fn test() {
    let x: &[isize] = &[1];
                   // ^^^^ adjustments: Deref(None), Borrow(Ref(Not)), Pointer(Unsize)
    let x: *const [isize] = &[1];
                         // ^^^^ adjustments: Deref(None), Borrow(RawPtr(Not)), Pointer(Unsize)
}
",
    );
}

#[test]
fn custom_coerce_unsized() {
    check(
        r#"
//- minicore: coerce_unsized
use core::{marker::Unsize, ops::CoerceUnsized};

struct A<T: ?Sized>(*const T);
struct B<T: ?Sized>(*const T);
struct C<T: ?Sized> { inner: *const T }

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<B<U>> for B<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<C<U>> for C<T> {}

fn foo1<T>(x: A<[T]>) -> A<[T]> { x }
fn foo2<T>(x: B<[T]>) -> B<[T]> { x }
fn foo3<T>(x: C<[T]>) -> C<[T]> { x }

fn test(a: A<[u8; 2]>, b: B<[u8; 2]>, c: C<[u8; 2]>) {
    let d = foo1(a);
              // ^ expected A<[{unknown}]>, got A<[u8; 2]>
    let e = foo2(b);
     // ^ type: B<[u8]>
    let f = foo3(c);
     // ^ type: C<[u8]>
}
"#,
    );
}

#[test]
fn if_coerce() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
fn foo<T>(x: &[T]) -> &[T] { x }
fn test() {
    let x = if true {
        foo(&[1])
         // ^^^^ adjustments: Deref(None), Borrow(Ref(Not)), Pointer(Unsize)
    } else {
        &[1]
    };
}
"#,
    );
}

#[test]
fn if_else_coerce() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
fn foo<T>(x: &[T]) -> &[T] { x }
fn test() {
    let x = if true {
        &[1]
    } else {
        foo(&[1])
    };
}
"#,
    )
}

#[test]
fn match_first_coerce() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
fn foo<T>(x: &[T]) -> &[T] { x }
fn test(i: i32) {
    let x = match i {
        2 => foo(&[2]),
              // ^^^^ adjustments: Deref(None), Borrow(Ref(Not)), Pointer(Unsize)
        1 => &[1],
        _ => &[3],
    };
}
"#,
    );
}

#[test]
fn match_second_coerce() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
fn foo<T>(x: &[T]) -> &[T] { loop {} }
                          // ^^^^^^^ adjustments: NeverToAny
fn test(i: i32) {
    let x = match i {
        1 => &[1],
        2 => foo(&[2]),
        _ => &[3],
    };
}
"#,
    );
}

#[test]
fn coerce_merge_one_by_one1() {
    cov_mark::check!(coerce_merge_fail_fallback);

    check(
        r"
fn test() {
    let t = &mut 1;
    let x = match 1 {
        1 => t as *mut i32,
        2 => t as &i32,
           //^^^^^^^^^ expected *mut i32, got &i32
        _ => t as *const i32,
          // ^^^^^^^^^^^^^^^ adjustments: Pointer(MutToConstPointer)

    };
    x;
  //^ type: *const i32

}
        ",
    );
}

#[test]
fn return_coerce_unknown() {
    check_types(
        r"
fn foo() -> u32 {
    return unknown;
         //^^^^^^^ u32
}
        ",
    );
}

#[test]
fn coerce_autoderef() {
    check_no_mismatches(
        r"
struct Foo;
fn takes_ref_foo(x: &Foo) {}
fn test() {
    takes_ref_foo(&Foo);
    takes_ref_foo(&&Foo);
    takes_ref_foo(&&&Foo);
}",
    );
}

#[test]
fn coerce_autoderef_generic() {
    check_no_mismatches(
        r#"
struct Foo;
fn takes_ref<T>(x: &T) -> T { *x }
fn test() {
    takes_ref(&Foo);
    takes_ref(&&Foo);
    takes_ref(&&&Foo);
}
"#,
    );
}

#[test]
fn coerce_autoderef_block() {
    check_no_mismatches(
        r#"
//- minicore: deref
struct String {}
impl core::ops::Deref for String { type Target = str; }
fn takes_ref_str(x: &str) {}
fn returns_string() -> String { loop {} }
fn test() {
    takes_ref_str(&{ returns_string() });
               // ^^^^^^^^^^^^^^^^^^^^^ adjustments: Deref(None), Deref(Some(OverloadedDeref(Not))), Borrow(Ref(Not))
}
"#,
    );
}

#[test]
fn closure_return_coerce() {
    check_no_mismatches(
        r"
fn foo() {
    let x = || {
        if true {
            return &1u32;
        }
        &&1u32
    };
}",
    );
}

#[test]
fn coerce_fn_item_to_fn_ptr() {
    check_no_mismatches(
        r"
fn foo(x: u32) -> isize { 1 }
fn test() {
    let f: fn(u32) -> isize = foo;
                           // ^^^ adjustments: Pointer(ReifyFnPointer)
    let f: unsafe fn(u32) -> isize = foo;
                                  // ^^^ adjustments: Pointer(ReifyFnPointer)
}",
    );
}

#[test]
fn coerce_fn_items_in_match_arms() {
    cov_mark::check!(coerce_fn_reification);

    check_types(
        r"
fn foo1(x: u32) -> isize { 1 }
fn foo2(x: u32) -> isize { 2 }
fn foo3(x: u32) -> isize { 3 }
fn test() {
    let x = match 1 {
        1 => foo1,
        2 => foo2,
        _ => foo3,
    };
    x;
  //^ fn(u32) -> isize
}",
    );
}

#[test]
fn coerce_closure_to_fn_ptr() {
    check_no_mismatches(
        r"
fn test() {
    let f: fn(u32) -> isize = |x| { 1 };
}",
    );
}

#[test]
fn coerce_placeholder_ref() {
    // placeholders should unify, even behind references
    check_no_mismatches(
        r"
struct S<T> { t: T }
impl<TT> S<TT> {
    fn get(&self) -> &TT {
        &self.t
    }
}",
    );
}

#[test]
fn coerce_unsize_array() {
    check_types(
        r#"
//- minicore: coerce_unsized
fn test() {
    let f: &[usize] = &[1, 2, 3];
                      //^ usize
}"#,
    );
}

#[test]
fn coerce_unsize_trait_object_simple() {
    check_types(
        r#"
//- minicore: coerce_unsized
trait Foo<T, U> {}
trait Bar<U, T, X>: Foo<T, U> {}
trait Baz<T, X>: Bar<usize, T, X> {}

struct S<T, X>;
impl<T, X> Foo<T, usize> for S<T, X> {}
impl<T, X> Bar<usize, T, X> for S<T, X> {}
impl<T, X> Baz<T, X> for S<T, X> {}

fn test() {
    let obj: &dyn Baz<i8, i16> = &S;
                                //^ S<i8, i16>
    let obj: &dyn Bar<_, i8, i16> = &S;
                                   //^ S<i8, i16>
    let obj: &dyn Foo<i8, _> = &S;
                              //^ S<i8, {unknown}>
}"#,
    );
}

#[test]
fn coerce_unsize_super_trait_cycle() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
trait A {}
trait B: C + A {}
trait C: B {}
trait D: C

struct S;
impl A for S {}
impl B for S {}
impl C for S {}
impl D for S {}

fn test() {
    let obj: &dyn D = &S;
    let obj: &dyn A = &S;
}
"#,
    );
}

#[test]
fn coerce_unsize_generic() {
    // FIXME: fix the type mismatches here
    check(
        r#"
//- minicore: coerce_unsized
struct Foo<T> { t: T };
struct Bar<T>(Foo<T>);

fn test() {
    let _: &Foo<[usize]> = &Foo { t: [1, 2, 3] };
                                   //^^^^^^^^^ expected [usize], got [usize; 3]
    let _: &Bar<[usize]> = &Bar(Foo { t: [1, 2, 3] });
                                       //^^^^^^^^^ expected [usize], got [usize; 3]
}
"#,
    );
}

#[test]
fn coerce_unsize_apit() {
    // FIXME: #8984
    check(
        r#"
//- minicore: coerce_unsized
trait Foo {}

fn test(f: impl Foo) {
    let _: &dyn Foo = &f;
                    //^^ expected &dyn Foo, got &impl Foo
}
        "#,
    );
}

#[test]
fn two_closures_lub() {
    check_types(
        r#"
fn foo(c: i32) {
    let add = |a: i32, b: i32| a + b;
    let sub = |a, b| a - b;
            //^^^^^^^^^^^^ |i32, i32| -> i32
    if c > 42 { add } else { sub };
  //^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ fn(i32, i32) -> i32
}
        "#,
    )
}

#[test]
fn match_diverging_branch_1() {
    check_types(
        r#"
enum Result<T> { Ok(T), Err }
fn parse<T>() -> T { loop {} }

fn test() -> i32 {
    let a = match parse() {
        Ok(val) => val,
        Err => return 0,
    };
    a
  //^ i32
}
        "#,
    )
}

#[test]
fn match_diverging_branch_2() {
    // same as 1 except for order of branches
    check_types(
        r#"
enum Result<T> { Ok(T), Err }
fn parse<T>() -> T { loop {} }

fn test() -> i32 {
    let a = match parse() {
        Err => return 0,
        Ok(val) => val,
    };
    a
  //^ i32
}
        "#,
    )
}

#[test]
fn panic_macro() {
    check_no_mismatches(
        r#"
mod panic {
    #[macro_export]
    pub macro panic_2015 {
        () => (
            $crate::panicking::panic()
        ),
    }
}

mod panicking {
    pub fn panic() -> ! { loop {} }
}

#[rustc_builtin_macro = "core_panic"]
macro_rules! panic {
    // Expands to either `$crate::panic::panic_2015` or `$crate::panic::panic_2021`
    // depending on the edition of the caller.
    ($($arg:tt)*) => {
        /* compiler built-in */
    };
}

fn main() {
    panic!()
}
        "#,
    );
}

#[test]
fn coerce_unsize_expected_type_1() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
fn main() {
    let foo: &[u32] = &[1, 2];
    let foo: &[u32] = match true {
        true => &[1, 2],
        false => &[1, 2, 3],
    };
    let foo: &[u32] = if true {
        &[1, 2]
    } else {
        &[1, 2, 3]
    };
}
        "#,
    );
}

#[test]
fn coerce_unsize_expected_type_2() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
struct InFile<T>;
impl<T> InFile<T> {
    fn with_value<U>(self, value: U) -> InFile<U> { InFile }
}
struct RecordField;
trait AstNode {}
impl AstNode for RecordField {}

fn takes_dyn(it: InFile<&dyn AstNode>) {}

fn test() {
    let x: InFile<()> = InFile;
    let n = &RecordField;
    takes_dyn(x.with_value(n));
}
        "#,
    );
}

#[test]
fn coerce_unsize_expected_type_3() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
enum Option<T> { Some(T), None }
struct RecordField;
trait AstNode {}
impl AstNode for RecordField {}

fn takes_dyn(it: Option<&dyn AstNode>) {}

fn test() {
    let x: InFile<()> = InFile;
    let n = &RecordField;
    takes_dyn(Option::Some(n));
}
        "#,
    );
}

#[test]
fn coerce_unsize_expected_type_4() {
    check_no_mismatches(
        r#"
//- minicore: coerce_unsized
use core::{marker::Unsize, ops::CoerceUnsized};

struct B<T: ?Sized>(*const T);
impl<T: ?Sized> B<T> {
    fn new(t: T) -> Self { B(&t) }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<B<U>> for B<T> {}

fn test() {
    let _: B<[isize]> = B::new({ [1, 2, 3] });
}
        "#,
    );
}

#[test]
fn coerce_array_elems_lub() {
    check_no_mismatches(
        r#"
fn f() {}
fn g() {}

fn test() {
    [f, g];
}
        "#,
    );
}
