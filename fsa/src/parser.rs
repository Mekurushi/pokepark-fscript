use crate::ast::{
    AddressingMode, BranchKind, BranchMode, Comparison, ConversionKind, Data, FloatOperation,
    Function, Instruction, IntegerOperation, Item, Linkage, MemoryWidth, ReturnKind, Script,
    ShiftOperation, Statement, StoreOperation, Terminator,
};
use crate::error::{ParseError, ParseResult};
use crate::lexer::token::{Mnemonic, TokenKind};

struct TokenStream {
    tokens: Vec<TokenKind>,
    cursor: usize,
}

impl TokenStream {
    fn new(tokens: Vec<TokenKind>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor)
    }

    fn peek_next(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor + 1)
    }

    fn advance(&mut self) -> Option<&TokenKind> {
        let token = self.tokens.get(self.cursor);
        if !self.is_at_end() {
            self.cursor += 1;
        }
        token
    }

    fn eat(&mut self, expected: &TokenKind) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &TokenKind, label: &'static str) -> ParseResult<()> {
        if self.eat(expected) {
            return Ok(());
        }
        Err(self.unexpected(label))
    }

    fn unexpected(&self, expected: &'static str) -> ParseError {
        match self.peek() {
            Some(token) => ParseError::UnexpectedToken {
                got: format!("{token:?}"),
                expected,
            },
            None => ParseError::UnexpectedEof { expected },
        }
    }

    fn eat_identifier(&mut self) -> Option<String> {
        let identifier = match self.peek()? {
            TokenKind::Identifier(identifier) => identifier.clone(),
            _ => return None,
        };
        self.advance();
        Some(identifier)
    }

    fn expect_identifier(&mut self) -> ParseResult<String> {
        self.eat_identifier()
            .ok_or_else(|| self.unexpected("identifier"))
    }

    fn expect_string(&mut self) -> ParseResult<String> {
        let value = match self.peek() {
            Some(TokenKind::String(value)) => value.clone(),
            _ => return Err(self.unexpected("string")),
        };
        self.advance();
        Ok(value)
    }
}

pub struct Parser {
    ts: TokenStream,
}

impl Parser {
    pub fn new(tokens: Vec<TokenKind>) -> Self {
        Self {
            ts: TokenStream::new(tokens),
        }
    }

    fn parse_integer(&mut self, expected: &'static str) -> ParseResult<i64> {
        let negative = self.ts.eat(&TokenKind::Minus);
        let value = match self.ts.peek() {
            Some(TokenKind::Integer(value)) => *value,
            _ => return Err(self.ts.unexpected(expected)),
        };
        self.ts.advance();

        if negative {
            value
                .checked_neg()
                .ok_or(ParseError::InvalidInteger { value, expected })
        } else {
            Ok(value)
        }
    }

    fn parse_i16(&mut self) -> ParseResult<i16> {
        let value = self.parse_integer("i16")?;
        i16::try_from(value).map_err(|_| ParseError::InvalidInteger {
            value,
            expected: "i16",
        })
    }

    fn parse_u8(&mut self) -> ParseResult<u8> {
        let value = self.parse_integer("u8")?;
        u8::try_from(value).map_err(|_| ParseError::InvalidInteger {
            value,
            expected: "u8",
        })
    }

    fn parse_u16(&mut self) -> ParseResult<u16> {
        let value = self.parse_integer("u16")?;
        u16::try_from(value).map_err(|_| ParseError::InvalidInteger {
            value,
            expected: "u16",
        })
    }

    fn parse_word(&mut self) -> ParseResult<u32> {
        let value = self.parse_integer("32-bit word")?;
        u32::try_from(value)
            .or_else(|_| i32::try_from(value).map(i32::cast_unsigned))
            .map_err(|_| ParseError::InvalidInteger {
                value,
                expected: "32-bit word",
            })
    }
    pub fn parse_script(mut self) -> ParseResult<Script> {
        let mut items = Vec::new();
        while !self.ts.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Script { items })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        match self.ts.peek() {
            Some(TokenKind::Extern) => self.parse_external(),
            Some(TokenKind::Export) => self.parse_function(Linkage::Exported).map(Item::Function),
            Some(TokenKind::Fn) => self.parse_function(Linkage::Internal).map(Item::Function),
            Some(TokenKind::Data) => self.parse_data().map(Item::Data),
            _ => Err(self.ts.unexpected("top-level declaration")),
        }
    }

    fn parse_external(&mut self) -> ParseResult<Item> {
        self.ts.expect(&TokenKind::Extern, "`extern`")?;
        let kind = self.ts.advance().cloned();
        let name = self.ts.expect_identifier()?;
        match kind {
            Some(TokenKind::Fn) => Ok(Item::ExternalFunction(name)),
            Some(TokenKind::Data) => Ok(Item::ExternalData(name)),
            Some(token) => Err(ParseError::UnexpectedToken {
                got: format!("{token:?}"),
                expected: "`fn` or `data`",
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "`fn` or `data`",
            }),
        }
    }

    fn parse_data(&mut self) -> ParseResult<Data> {
        self.ts.expect(&TokenKind::Data, "`data`")?;
        let name = self.ts.expect_identifier()?;
        self.ts.expect(&TokenKind::Equal, "`=`")?;
        self.ts.expect(&TokenKind::LeftBracket, "`[`")?;

        let mut words = Vec::new();
        if self.ts.peek() != Some(&TokenKind::RightBracket) {
            words.push(self.parse_word()?);
            while self.ts.eat(&TokenKind::Comma) {
                words.push(self.parse_word()?);
            }
        }

        self.ts.expect(&TokenKind::RightBracket, "`]`")?;
        Ok(Data { name, words })
    }

    fn parse_function(&mut self, linkage: Linkage) -> ParseResult<Function> {
        if linkage == Linkage::Exported {
            self.ts.expect(&TokenKind::Export, "`export`")?;
        }
        self.ts.expect(&TokenKind::Fn, "`fn`")?;
        let name = self.ts.expect_identifier()?;
        self.ts.expect(&TokenKind::Colon, "`:`")?;

        let mut body = Vec::new();
        while !Self::is_item_start(self.ts.peek()) {
            body.push(self.parse_statement()?);
        }

        Ok(Function {
            name,
            linkage,
            body,
        })
    }

    fn is_item_start(token: Option<&TokenKind>) -> bool {
        matches!(
            token,
            None | Some(TokenKind::Extern | TokenKind::Export | TokenKind::Fn | TokenKind::Data)
        )
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        if matches!(self.ts.peek(), Some(TokenKind::Identifier(_)))
            && self.ts.peek_next() == Some(&TokenKind::Colon)
        {
            let name = self.ts.expect_identifier()?;
            self.ts.expect(&TokenKind::Colon, "`:`")?;
            return Ok(Statement::Label(name));
        }

        self.parse_instruction()
    }

    fn parse_syscall(&mut self) -> ParseResult<Statement> {
        let argument_count = match self.ts.peek() {
            Some(TokenKind::Mnemonic(Mnemonic::Syscall(argument_count))) => *argument_count,
            _ => return Err(self.ts.unexpected("syscall")),
        };
        self.ts.advance();
        let page = self.parse_u8()?;
        self.ts.expect(&TokenKind::Colon, "`:`")?;
        Ok(Statement::Instruction(Instruction::Syscall {
            argument_count,
            page,
            function: self.parse_u16()?,
        }))
    }

    fn parse_instruction(&mut self) -> ParseResult<Statement> {
        let mnemonic = match self.ts.peek() {
            Some(TokenKind::Mnemonic(Mnemonic::Syscall(_))) => return self.parse_syscall(),
            Some(TokenKind::Mnemonic(mnemonic)) => *mnemonic,
            _ => return Err(self.ts.unexpected("instruction")),
        };
        self.ts.advance();
        let instruction = |instruction| Ok(Statement::Instruction(instruction));
        let terminator = |terminator| Ok(Statement::Terminator(terminator));

        match mnemonic {
            Mnemonic::Syscall(_) => unreachable!(),
            Mnemonic::Delay => instruction(Instruction::Delay(self.parse_i16()?)),
            Mnemonic::DelayFromStack => instruction(Instruction::DelayFromStack),
            Mnemonic::DelayIfNonZero => instruction(Instruction::DelayIfNonZero),
            Mnemonic::Call => instruction(Instruction::Call(self.ts.expect_identifier()?)),
            Mnemonic::GrowStack => instruction(Instruction::GrowStack(self.parse_u16()?)),
            Mnemonic::ShrinkStack => instruction(Instruction::ShrinkStack(self.parse_u16()?)),
            Mnemonic::LoadFrameSlot => instruction(Instruction::LoadFrameSlot(self.parse_i16()?)),
            Mnemonic::LoadFrameSlotAddress => {
                instruction(Instruction::LoadFrameSlotAddress(self.parse_i16()?))
            }
            Mnemonic::StoreFrameSlot => instruction(Instruction::StoreFrameSlot(self.parse_i16()?)),
            Mnemonic::AddToFrameSlot => instruction(Instruction::AddToFrameSlot(self.parse_i16()?)),
            Mnemonic::SubtractFromFrameSlot => {
                instruction(Instruction::SubtractFromFrameSlot(self.parse_i16()?))
            }
            Mnemonic::LoadRel => instruction(Instruction::LoadRel(self.parse_i16()?)),
            Mnemonic::PushConstant => instruction(Instruction::PushConstant(self.parse_word()?)),
            Mnemonic::PushResult => instruction(Instruction::PushResult),
            Mnemonic::LoadString => instruction(Instruction::LoadString(self.ts.expect_string()?)),
            Mnemonic::Add => instruction(Instruction::Integer(IntegerOperation::Add)),
            Mnemonic::Subtract => instruction(Instruction::Integer(IntegerOperation::Subtract)),
            Mnemonic::Multiply => instruction(Instruction::Integer(IntegerOperation::Multiply)),
            Mnemonic::Divide => instruction(Instruction::Integer(IntegerOperation::Divide)),
            Mnemonic::Remainder => instruction(Instruction::Integer(IntegerOperation::Remainder)),
            Mnemonic::And => instruction(Instruction::Integer(IntegerOperation::And)),
            Mnemonic::Or => instruction(Instruction::Integer(IntegerOperation::Or)),
            Mnemonic::Xor => instruction(Instruction::Integer(IntegerOperation::Xor)),
            Mnemonic::Not => instruction(Instruction::Integer(IntegerOperation::Not)),
            Mnemonic::IsZero => instruction(Instruction::Integer(IntegerOperation::IsZero)),
            Mnemonic::Negate => instruction(Instruction::Integer(IntegerOperation::Negate)),
            Mnemonic::FloatAdd => instruction(Instruction::Float(FloatOperation::Add)),
            Mnemonic::FloatSubtract => instruction(Instruction::Float(FloatOperation::Subtract)),
            Mnemonic::FloatMultiply => instruction(Instruction::Float(FloatOperation::Multiply)),
            Mnemonic::FloatDivide => instruction(Instruction::Float(FloatOperation::Divide)),
            Mnemonic::FloatIsZero => instruction(Instruction::Float(FloatOperation::IsZero)),
            Mnemonic::FloatNegate => instruction(Instruction::Float(FloatOperation::Negate)),
            Mnemonic::Equal => instruction(Instruction::IntegerCompare(Comparison::Equal)),
            Mnemonic::NotEqual => instruction(Instruction::IntegerCompare(Comparison::NotEqual)),
            Mnemonic::LessThan => instruction(Instruction::IntegerCompare(Comparison::LessThan)),
            Mnemonic::GreaterThan => {
                instruction(Instruction::IntegerCompare(Comparison::GreaterThan))
            }
            Mnemonic::LessOrEqual => {
                instruction(Instruction::IntegerCompare(Comparison::LessOrEqual))
            }
            Mnemonic::GreaterOrEqual => {
                instruction(Instruction::IntegerCompare(Comparison::GreaterOrEqual))
            }
            Mnemonic::FloatEqual => instruction(Instruction::FloatCompare(Comparison::Equal)),
            Mnemonic::FloatNotEqual => instruction(Instruction::FloatCompare(Comparison::NotEqual)),
            Mnemonic::FloatLessThan => instruction(Instruction::FloatCompare(Comparison::LessThan)),
            Mnemonic::FloatGreaterThan => {
                instruction(Instruction::FloatCompare(Comparison::GreaterThan))
            }
            Mnemonic::FloatLessOrEqual => {
                instruction(Instruction::FloatCompare(Comparison::LessOrEqual))
            }
            Mnemonic::FloatGreaterOrEqual => {
                instruction(Instruction::FloatCompare(Comparison::GreaterOrEqual))
            }
            Mnemonic::ShiftLeft => instruction(Instruction::Shift(ShiftOperation::Left)),
            Mnemonic::ShiftRightArithmetic => {
                instruction(Instruction::Shift(ShiftOperation::RightArithmetic))
            }
            Mnemonic::ShiftRightLogical => {
                instruction(Instruction::Shift(ShiftOperation::RightLogical))
            }
            Mnemonic::LoadAddress => {
                instruction(Instruction::LoadAddress(self.ts.expect_identifier()?))
            }
            Mnemonic::LoadByte => instruction(load(MemoryWidth::Byte, AddressingMode::Direct)),
            Mnemonic::LoadHalfword => {
                instruction(load(MemoryWidth::Halfword, AddressingMode::Direct))
            }
            Mnemonic::LoadWord => instruction(load(MemoryWidth::Word, AddressingMode::Direct)),
            Mnemonic::LoadByteIndexed => {
                instruction(load(MemoryWidth::Byte, AddressingMode::Indexed))
            }
            Mnemonic::LoadHalfwordIndexed => {
                instruction(load(MemoryWidth::Halfword, AddressingMode::Indexed))
            }
            Mnemonic::LoadWordIndexed => {
                instruction(load(MemoryWidth::Word, AddressingMode::Indexed))
            }
            Mnemonic::StoreByte => instruction(store(
                MemoryWidth::Byte,
                AddressingMode::Direct,
                StoreOperation::Assign,
            )),
            Mnemonic::StoreHalfword => instruction(store(
                MemoryWidth::Halfword,
                AddressingMode::Direct,
                StoreOperation::Assign,
            )),
            Mnemonic::StoreWord => instruction(store(
                MemoryWidth::Word,
                AddressingMode::Direct,
                StoreOperation::Assign,
            )),
            Mnemonic::StoreByteIndexed => instruction(store(
                MemoryWidth::Byte,
                AddressingMode::Indexed,
                StoreOperation::Assign,
            )),
            Mnemonic::StoreHalfwordIndexed => instruction(store(
                MemoryWidth::Halfword,
                AddressingMode::Indexed,
                StoreOperation::Assign,
            )),
            Mnemonic::StoreWordIndexed => instruction(store(
                MemoryWidth::Word,
                AddressingMode::Indexed,
                StoreOperation::Assign,
            )),
            Mnemonic::AddByte => instruction(store(
                MemoryWidth::Byte,
                AddressingMode::Direct,
                StoreOperation::Add,
            )),
            Mnemonic::AddHalfword => instruction(store(
                MemoryWidth::Halfword,
                AddressingMode::Direct,
                StoreOperation::Add,
            )),
            Mnemonic::AddWord => instruction(store(
                MemoryWidth::Word,
                AddressingMode::Direct,
                StoreOperation::Add,
            )),
            Mnemonic::AddByteIndexed => instruction(store(
                MemoryWidth::Byte,
                AddressingMode::Indexed,
                StoreOperation::Add,
            )),
            Mnemonic::AddHalfwordIndexed => instruction(store(
                MemoryWidth::Halfword,
                AddressingMode::Indexed,
                StoreOperation::Add,
            )),
            Mnemonic::AddWordIndexed => instruction(store(
                MemoryWidth::Word,
                AddressingMode::Indexed,
                StoreOperation::Add,
            )),
            Mnemonic::SubtractByte => instruction(store(
                MemoryWidth::Byte,
                AddressingMode::Direct,
                StoreOperation::Subtract,
            )),
            Mnemonic::SubtractHalfword => instruction(store(
                MemoryWidth::Halfword,
                AddressingMode::Direct,
                StoreOperation::Subtract,
            )),
            Mnemonic::SubtractWord => instruction(store(
                MemoryWidth::Word,
                AddressingMode::Direct,
                StoreOperation::Subtract,
            )),
            Mnemonic::SubtractByteIndexed => instruction(store(
                MemoryWidth::Byte,
                AddressingMode::Indexed,
                StoreOperation::Subtract,
            )),
            Mnemonic::SubtractHalfwordIndexed => instruction(store(
                MemoryWidth::Halfword,
                AddressingMode::Indexed,
                StoreOperation::Subtract,
            )),
            Mnemonic::SubtractWordIndexed => instruction(store(
                MemoryWidth::Word,
                AddressingMode::Indexed,
                StoreOperation::Subtract,
            )),
            Mnemonic::IntegerToFloat => instruction(Instruction::Convert {
                kind: ConversionKind::IntegerToFloat,
                stack_offset: self.parse_i16()?,
            }),
            Mnemonic::FloatToInteger => instruction(Instruction::Convert {
                kind: ConversionKind::FloatToInteger,
                stack_offset: self.parse_i16()?,
            }),
            Mnemonic::Debug => instruction(Instruction::Debug {
                subtype: self.parse_u8()?,
                operand: self.parse_u16()?,
            }),
            Mnemonic::Jump => terminator(Terminator::Jump(self.ts.expect_identifier()?)),
            Mnemonic::JumpIfZero => self.finish_branch(BranchKind::IfZero(BranchMode::Normal)),
            Mnemonic::JumpIfZeroPause => self.finish_branch(BranchKind::IfZero(BranchMode::Pause)),
            Mnemonic::JumpIfZeroSet => self.finish_branch(BranchKind::IfZero(BranchMode::Set)),
            Mnemonic::JumpIfNonZero => {
                self.finish_branch(BranchKind::IfNonZero(BranchMode::Normal))
            }
            Mnemonic::JumpIfNonZeroPause => {
                self.finish_branch(BranchKind::IfNonZero(BranchMode::Pause))
            }
            Mnemonic::JumpIfNonZeroSet => {
                self.finish_branch(BranchKind::IfNonZero(BranchMode::Set))
            }
            Mnemonic::JumpIfEqual => self.finish_branch(BranchKind::IfEqual),
            Mnemonic::JumpIfEqualImmediate => {
                let immediate = self.parse_u8()?;
                self.finish_branch(BranchKind::IfEqualImmediate(immediate))
            }
            Mnemonic::Return => terminator(Terminator::Return {
                kind: ReturnKind::None,
                frame_slots: self.parse_u16()?,
            }),
            Mnemonic::ReturnValue => terminator(Terminator::Return {
                kind: ReturnKind::Value,
                frame_slots: self.parse_u16()?,
            }),
            Mnemonic::Exit1 => terminator(Terminator::Exit1),
            Mnemonic::Exit2 => terminator(Terminator::Exit2),
        }
    }

    fn finish_branch(&mut self, kind: BranchKind) -> ParseResult<Statement> {
        Ok(Statement::Terminator(Terminator::Branch {
            kind,
            target: self.ts.expect_identifier()?,
        }))
    }
}

fn load(width: MemoryWidth, addressing: AddressingMode) -> Instruction {
    Instruction::Load { width, addressing }
}

fn store(width: MemoryWidth, addressing: AddressingMode, operation: StoreOperation) -> Instruction {
    Instruction::Store {
        width,
        addressing,
        operation,
    }
}
