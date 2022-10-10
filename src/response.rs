use serde::{
    de::{self, Error as SeError, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize,
};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use zbus::zvariant::{OwnedValue, Signature, Type};
#[derive(Debug, Copy, PartialEq, Eq, Hash, Clone)]
/// An error returned a portal request caused by either the user cancelling the
/// request or something else.
pub enum ResponseError {
    /// The user canceled the request.
    Cancelled,
    /// Something else happened.
    Other,
}

impl std::error::Error for ResponseError {}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("Cancelled"),
            Self::Other => f.write_str("Other"),
        }
    }
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Type)]
#[doc(hidden)]
enum ResponseType {
    /// Success, the request is carried out.
    Success = 0,
    /// The user cancelled the interaction.
    Cancelled = 1,
    /// The user interaction was ended in some other way.
    Other = 2,
}

#[doc(hidden)]
impl From<ResponseError> for ResponseType {
    fn from(err: ResponseError) -> Self {
        match err {
            ResponseError::Other => Self::Other,
            ResponseError::Cancelled => Self::Cancelled,
        }
    }
}


#[derive(Debug)]
pub(crate) enum Response<T>
where
    T: for<'de> Deserialize<'de> + Type,
{
    /// Success, the request is carried out.
    Ok(T),
    /// The user cancelled the request or something else happened.
    Err(ResponseError),
}

impl<T> Type for Response<T>
where
    T: for<'de> Deserialize<'de> + Type,
{
    fn signature() -> Signature<'static> {
        <(ResponseType, HashMap<&str, OwnedValue>)>::signature()
    }
}

impl<'de, T> Deserialize<'de> for Response<T>
where
    T: for<'d> Deserialize<'d> + Type,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResponseVisitor<T>(PhantomData<fn() -> (ResponseType, T)>);

        impl<'de, T> Visitor<'de> for ResponseVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = (ResponseType, Option<T>);

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "a tuple composed of the response status along with the response"
                )
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let type_: ResponseType = seq.next_element()?.ok_or_else(|| A::Error::custom(
                    "Failed to deserialize the response. Expected a numeric (u) value as the first item of the returned tuple",
                ))?;
                if type_ == ResponseType::Success {
                    let data: T = seq.next_element()?.ok_or_else(|| A::Error::custom(
                        "Failed to deserialize the response. Expected a vardict (a{sv}) with the returned results",
                    ))?;
                    Ok((type_, Some(data)))
                } else {
                    Ok((type_, None))
                }
            }
        }

        let visitor = ResponseVisitor::<T>(PhantomData);
        let response: (ResponseType, Option<T>) = deserializer.deserialize_tuple(2, visitor)?;
        Ok(response.into())
    }
}

impl<T> Serialize for Response<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Type,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_tuple(2)?;
        match self {
            Self::Err(err) => {
                map.serialize_element(&ResponseType::from(*err))?;
                map.serialize_element(&BasicResponse::default())?;
            }
            Self::Ok(response) => {
                map.serialize_element(&ResponseType::Success)?;
                map.serialize_element(response)?;
            }
        };
        map.end()
    }
}

#[doc(hidden)]
impl<T> From<(ResponseType, Option<T>)> for Response<T>
where
    T: for<'de> Deserialize<'de> + Type,
{
    fn from(f: (ResponseType, Option<T>)) -> Self {
        match f.0 {
            ResponseType::Success => {
                Response::Ok(f.1.expect("Expected a valid response, found nothing."))
            }
            ResponseType::Cancelled => Response::Err(ResponseError::Cancelled),
            ResponseType::Other => Response::Err(ResponseError::Other),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Type)]
/// The most basic response. Used when only the status of the request is what we
/// receive as a response.
pub(crate) struct BasicResponse(HashMap<String, OwnedValue>);

impl Debug for BasicResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BasicResponse").finish()
    }
}
