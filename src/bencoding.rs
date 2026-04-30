use std::{fmt::Display, str::Chars};

use anyhow::Result;

pub enum Bencoding {
    String(String),
    Integer(i64),
    List(Vec<Bencoding>),
    Dictionary(Vec<(String, Bencoding)>),
}

impl Display for Bencoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "\"{}\"", s),
            Self::Integer(i) => write!(f, "{}", i),
            Self::List(l) => {
                write!(f, "[")?;
                for (i, val) in l.iter().enumerate() {
                    if i + 1 == l.len() {
                        write!(f, "{}", val)?;
                    } else {
                        write!(f, "{},", val)?;
                    }
                }
                write!(f, "]")
            }
            Self::Dictionary(d) => {
                write!(f, "{{")?;
                for (i, (key, val)) in d.iter().enumerate() {
                    if i + 1 == d.len() {
                        write!(f, "\"{}\":{}", key, val)?;
                    } else {
                        write!(f, "\"{}\":{},", key, val)?;
                    }
                }
                write!(f, "}}")
            }
        }
    }
}

impl Bencoding {
    pub fn decode(iter: &mut Chars) -> Result<Option<Self>> {
        if let Some(c) = iter.next() {
            match c {
                'e' => return Ok(None),
                'i' => {
                    let num = read_util(iter, 'e')?;
                    let num: i64 = num.parse()?;
                    return Ok(Some(Self::Integer(num)));
                }
                'l' => {
                    let mut list = Vec::new();
                    while let Some(val) = Self::decode(iter)? {
                        list.push(val);
                    }
                    return Ok(Some(Self::List(list)));
                }
                'd' => {
                    let mut dict = Vec::new();
                    while let Some(encoding) = Self::decode(iter)? {
                        let Self::String(key) = encoding else {
                            anyhow::bail!("key in dictionary must be string")
                        };
                        let Some(val) = Self::decode(iter)? else {
                            anyhow::bail!("no corresponding value to key {key} in dictionary")
                        };
                        dict.push((key, val));
                    }
                    return Ok(Some(Self::Dictionary(dict)));
                }
                c => {
                    let mut len = String::from(c);
                    len.push_str(&read_util(iter, ':')?);
                    let mut len: u64 = len.parse()?;
                    let mut s = String::new();
                    while len > 0
                        && let Some(c) = iter.next()
                    {
                        s.push(c);
                        len -= 1;
                    }
                    anyhow::ensure!(len == 0);
                    return Ok(Some(Self::String(s)));
                }
            }
        }
        anyhow::bail!("Cannot decode")
    }
}

fn read_util(iter: &mut Chars, delimiter: char) -> Result<String> {
    let mut res = String::new();
    for c in iter.by_ref() {
        res.push(c);
        if c == delimiter {
            break;
        }
    }
    anyhow::ensure!(res.ends_with(delimiter));
    res.pop();
    Ok(res)
}
