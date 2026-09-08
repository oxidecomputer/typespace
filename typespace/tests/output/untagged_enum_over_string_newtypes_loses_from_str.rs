pub struct Literal(pub ::std::string::String);
impl ::std::ops::Deref for Literal {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Literal> for ::std::string::String {
    fn from(value: Literal) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::string::String> for Literal {
    fn from(value: ::std::string::String) -> Self {
        Self(value)
    }
}
impl ::std::fmt::Display for Literal {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Literal {
    type Err = ::std::convert::Infallible;
    fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self(value.to_string()))
    }
}
pub struct Reference(pub ::std::string::String);
impl ::std::ops::Deref for Reference {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<Reference> for ::std::string::String {
    fn from(value: Reference) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::string::String> for Reference {
    fn from(value: ::std::string::String) -> Self {
        Self(value)
    }
}
impl ::std::fmt::Display for Reference {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
impl ::std::str::FromStr for Reference {
    type Err = ::std::convert::Infallible;
    fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self(value.to_string()))
    }
}
pub enum ReferenceOrLiteral {
    Reference(Reference),
    Literal(Literal),
}
impl ::std::fmt::Display for ReferenceOrLiteral {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            Self::Reference(x) => x.fmt(f),
            Self::Literal(x) => x.fmt(f),
        }
    }
}
impl ::std::convert::From<Reference> for ReferenceOrLiteral {
    fn from(value: Reference) -> Self {
        Self::Reference(value)
    }
}
impl ::std::convert::From<Literal> for ReferenceOrLiteral {
    fn from(value: Literal) -> Self {
        Self::Literal(value)
    }
}
