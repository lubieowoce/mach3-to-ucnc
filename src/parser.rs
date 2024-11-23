enum Line {
    Command { id: CommandId, args: Vec<Arg> },
    SetRegister { target: RegisterId, value: Expr },
    Blank,
}

struct CommandId(CommandType, usize, Option<usize>);

enum CommandType {
    G,
    M,
}

enum Arg {
    Positional { expr: Expr },
    // https://reprap.org/wiki/G-code#Fields
    Keyword { name: KeywordName, expr: Expr },
}

type RegisterId = usize;
enum Expr {
    Register(RegisterId),
    Int(i64),     // TODO: maybe use https://docs.rs/rust_decimal/latest/rust_decimal/
    Decimal(f64), // TODO: maybe use https://docs.rs/rust_decimal/latest/rust_decimal/
    Bracket(Box<Expr>),
    Arithmetic(Box<ArithmeticExpr>),
}

enum ArithmeticExpr {
    Expr(Box<Expr>),
    Binop {
        op: ArithmeticOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
}

struct KeywordName(char);

enum Common {
    X,
    Y,
    Z,
    F,
}
