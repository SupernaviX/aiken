use anyhow::anyhow;

use flat::{
    de::{Decode, Decoder},
    en::{Encode, Encoder},
    Flat,
};

use crate::{
    ast::{Constant, Program, Term},
    builtins::DefaultFunction,
};

const BUILTIN_TAG_WIDTH: u32 = 7;
const CONST_TAG_WIDTH: u32 = 4;
const TERM_TAG_WIDTH: u32 = 4;

impl<'b> Flat<'b> for Program {}

impl Program {
    // convenient so that people don't need to depend on the flat crate
    // directly to call programs flat function
    pub fn to_flat(&self) -> anyhow::Result<Vec<u8>> {
        self.flat().map_err(|err| anyhow!("{}", err))
    }

    pub fn flat_hex(&self) -> anyhow::Result<String> {
        let bytes = self.flat().map_err(|err| anyhow!("{}", err))?;

        let hex = hex::encode(&bytes);

        Ok(hex)
    }
}

impl Encode for Program {
    fn encode(&self, e: &mut Encoder) -> Result<(), String> {
        let (major, minor, patch) = self.version;

        major.encode(e)?;
        minor.encode(e)?;
        patch.encode(e)?;

        self.term.encode(e)?;

        Ok(())
    }
}

impl<'b> Decode<'b> for Program {
    fn decode(d: &mut Decoder) -> Result<Self, String> {
        let version = (usize::decode(d)?, usize::decode(d)?, usize::decode(d)?);
        let term = Term::decode(d)?;
        Ok(Program { version, term })
    }
}

impl Encode for Term {
    fn encode(&self, e: &mut Encoder) -> Result<(), String> {
        // still need annotation but here we have the term tags
        match self {
            Term::Var(name) => {
                encode_term_tag(0, e)?;
                name.encode(e)?;
            }
            Term::Delay(term) => {
                encode_term_tag(1, e)?;
                term.encode(e)?;
            }
            Term::Lambda {
                parameter_name: _,
                body: _,
            } => {
                encode_term_tag(2, e)?;
                // need to create encoding for Binder
                todo!();
            }
            Term::Apply { function, argument } => {
                encode_term_tag(3, e)?;
                function.encode(e)?;
                argument.encode(e)?;
            }

            Term::Constant(constant) => {
                encode_term_tag(4, e)?;
                constant.encode(e)?;
            }

            Term::Force(term) => {
                encode_term_tag(5, e)?;
                term.encode(e)?;
            }

            Term::Error => {
                encode_term_tag(6, e)?;
            }
            Term::Builtin(builtin) => {
                encode_term_tag(7, e)?;

                builtin.encode(e)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b> for Term {
    fn decode(d: &mut Decoder) -> Result<Self, String> {
        match decode_term_tag(d)? {
            0 => Ok(Term::Var(String::decode(d)?)),
            1 => Ok(Term::Delay(Box::new(Term::decode(d)?))),
            2 => todo!(),
            3 => Ok(Term::Apply {
                function: Box::new(Term::decode(d)?),
                argument: Box::new(Term::decode(d)?),
            }),
            // Need size limit for Constant
            4 => Ok(Term::Constant(Constant::decode(d)?)),
            5 => Ok(Term::Force(Box::new(Term::decode(d)?))),
            6 => Ok(Term::Error),
            7 => Ok(Term::Builtin(DefaultFunction::decode(d)?)),
            x => Err(format!("Unknown term constructor tag: {}", x)),
        }
    }
}

impl Encode for &Constant {
    fn encode(&self, e: &mut Encoder) -> Result<(), String> {
        match self {
            Constant::Integer(i) => {
                encode_constant(0, e)?;
                i.encode(e)?;
            }
            Constant::ByteString(bytes) => {
                encode_constant(1, e)?;
                bytes.encode(e)?;
            }
            Constant::String(s) => {
                encode_constant(2, e)?;
                s.as_bytes().encode(e)?;
            }
            // there is no char constant tag
            Constant::Char(c) => {
                c.encode(e)?;

                let mut b = [0; 4];

                let s = c.encode_utf8(&mut b);

                s.as_bytes().encode(e)?;
            }
            Constant::Unit => encode_constant(3, e)?,
            Constant::Bool(b) => {
                encode_constant(4, e)?;
                b.encode(e)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b> for Constant {
    fn decode(d: &mut Decoder) -> Result<Self, String> {
        match decode_constant(d)? {
            0 => Ok(Constant::Integer(isize::decode(d)?)),
            1 => Ok(Constant::ByteString(Vec::<u8>::decode(d)?)),
            2 => Ok(Constant::String(
                String::from_utf8(Vec::<u8>::decode(d)?).unwrap(),
            )),
            3 => Ok(Constant::Unit),
            4 => Ok(Constant::Bool(bool::decode(d)?)),
            x => Err(format!("Unknown constant constructor tag: {}", x)),
        }
    }
}

impl Encode for DefaultFunction {
    fn encode(&self, e: &mut flat::en::Encoder) -> Result<(), String> {
        e.bits(BUILTIN_TAG_WIDTH as i64, self.clone() as u8);

        Ok(())
    }
}

impl<'b> Decode<'b> for DefaultFunction {
    fn decode(d: &mut Decoder) -> Result<Self, String> {
        let builtin_tag = d.bits8(BUILTIN_TAG_WIDTH as usize)?;
        builtin_tag.try_into()
    }
}

fn encode_term_tag(tag: u8, e: &mut Encoder) -> Result<(), String> {
    safe_encode_bits(TERM_TAG_WIDTH, tag, e)
}

fn decode_term_tag(d: &mut Decoder) -> Result<u8, String> {
    d.bits8(TERM_TAG_WIDTH as usize)
}

fn safe_encode_bits(num_bits: u32, byte: u8, e: &mut Encoder) -> Result<(), String> {
    if 2_u8.pow(num_bits) < byte {
        Err(format!(
            "Overflow detected, cannot fit {} in {} bits.",
            byte, num_bits
        ))
    } else {
        e.bits(num_bits as i64, byte);
        Ok(())
    }
}

pub fn encode_constant(tag: u8, e: &mut Encoder) -> Result<(), String> {
    e.encode_list_with(encode_constant_tag, [tag].to_vec())
}

pub fn decode_constant(d: &mut Decoder) -> Result<u8, String> {
    let u8_list = d.decode_list_with(decode_constant_tag)?;
    if u8_list.len() > 1 {
        Err(
            "Improper encoding on constant tag. Should be list of one item encoded in 4 bits"
                .to_string(),
        )
    } else {
        Ok(u8_list[0])
    }
}

pub fn encode_constant_tag(tag: u8, e: &mut Encoder) -> Result<(), String> {
    safe_encode_bits(CONST_TAG_WIDTH, tag, e)
}

pub fn decode_constant_tag(d: &mut Decoder) -> Result<u8, String> {
    d.bits8(CONST_TAG_WIDTH as usize)
}

#[cfg(test)]
mod test {
    use flat::Flat;

    use super::{Constant, Program, Term};

    #[test]
    fn flat_encode_integer() {
        let program = Program {
            version: (11, 22, 33),
            term: Term::Constant(Constant::Integer(11)),
        };

        let bytes = program.to_flat().unwrap();

        assert_eq!(
            bytes,
            vec![0b00001011, 0b00010110, 0b00100001, 0b01001000, 0b00000101, 0b10000001]
        )
    }

    #[test]
    fn flat_decode_integer() {
        let flat_encoded = vec![
            0b00001011, 0b00010110, 0b00100001, 0b01001000, 0b00000101, 0b10000001,
        ];
        let expected_program = Program {
            version: (11, 22, 33),
            term: Term::Constant(Constant::Integer(11)),
        };

        let actual_program: Program = Program::unflat(flat_encoded.as_slice()).unwrap();

        assert_eq!(actual_program, expected_program)
    }
}
