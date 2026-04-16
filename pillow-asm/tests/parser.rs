use pillow_asm::ProgramParser;

#[test]
fn parse_full_instruction_set() {
    let src = r#"
start:
    nop
    const 1
    add
    sub
    mul
    div
    return

    neg
    not

    eq
    ne
    lt
    le
    gt
    ge

    jmp end
    jmp_if_false end
    jmp_if_true end

    pop
    dup

    make_frame 2
    get_local 1
    set_local 1

    call 128, 2

    enter_nogc
    exit_nogc

    alloc 16, 1
    load
    store
    loadb
    storeb

end:
    return
"#;

    let ast = ProgramParser::new().parse(src).unwrap();

    println!("{:#?}", ast);
}

#[test]
fn parse_full_program() {
    let src = r#"
start:
    nop
    const 1
    add
    sub
    mul
    div
    return

    neg
    not

    eq
    ne
    lt
    le
    gt
    ge

    jmp end
    jmp_if_false end
    jmp_if_true end

    pop
    dup

    make_frame 2
    get_local 1
    set_local 1

    call 128, 2

    enter_nogc
    exit_nogc

    alloc 16, 1
    load
    store
    loadb
    storeb

end:
    return
"#;

    let ast = ProgramParser::new().parse(src);
    assert!(ast.is_ok());
}

#[test]
fn parse_single_instruction() {
    let ast = ProgramParser::new().parse("nop");
    assert!(ast.is_ok());
}

#[test]
fn parse_label_only() {
    let ast = ProgramParser::new().parse("start:");
    assert!(ast.is_ok());
}

#[test]
fn fail_unknown_instruction() {
    let ast = ProgramParser::new().parse("foobar");
    assert!(ast.is_err());
}

#[test]
fn fail_missing_operand() {
    let ast = ProgramParser::new().parse("const");
    assert!(ast.is_err());
}

#[test]
fn fail_extra_operand() {
    let ast = ProgramParser::new().parse("const 1 2");
    assert!(ast.is_err());
}

#[test]
fn fail_wrong_operand_type() {
    let ast = ProgramParser::new().parse("const foo");
    assert!(ast.is_err());
}

#[test]
fn fail_missing_comma_call() {
    let ast = ProgramParser::new().parse("call 128 2");
    assert!(ast.is_err());
}

#[test]
fn fail_missing_comma_alloc() {
    let ast = ProgramParser::new().parse("alloc 16 1");
    assert!(ast.is_err());
}

#[test]
fn fail_invalid_label() {
    let ast = ProgramParser::new().parse("123abc:");
    assert!(ast.is_err());
}

#[test]
fn fail_label_without_colon() {
    let ast = ProgramParser::new().parse("start");
    assert!(ast.is_err());
}

#[test]
fn fail_random_garbage() {
    let ast = ProgramParser::new().parse("@@@ ???");
    assert!(ast.is_err());
}

#[test]
fn fail_u8_overflow() {
    let res = ProgramParser::new().parse("const 999");
    assert!(res.is_err());
}

#[test]
fn fail_u16_overflow() {
    let res = ProgramParser::new().parse("alloc 70000, 1");
    assert!(res.is_err());
}

#[test]
fn parse_empty() {
    let ast = ProgramParser::new().parse("");
    assert!(ast.is_ok());
}

#[test]
fn parse_whitespace_variants() {
    let src = "   \n  const   1   \n   add   ";
    let ast = ProgramParser::new().parse(src);
    assert!(ast.is_ok());
}

#[test]
fn parse_multiple_labels() {
    let src = r#"
a:
b:
c:
    nop
"#;

    let ast = ProgramParser::new().parse(src);
    assert!(ast.is_ok());
}
