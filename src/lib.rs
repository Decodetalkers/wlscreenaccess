pub mod response;
use zbus::{
    export::futures_util::StreamExt,
    zvariant::{DeserializeDict, OwnedObjectPath, SerializeDict, Type},
};

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use zbus::{dbus_proxy, names::OwnedMemberName, Connection};

#[derive(Serialize, Deserialize, Type, Debug)]
pub struct HandleToken(OwnedMemberName);
impl Default for HandleToken {
    fn default() -> Self {
        let mut rng = thread_rng();
        let token: String = (&mut rng)
            .sample_iter(Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        HandleToken::try_from(format!("ashpd_{}", token)).unwrap()
    }
}
#[derive(Debug)]
pub struct HandleInvalidCharacter(char);

impl std::fmt::Display for HandleInvalidCharacter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Invalid Character {}", self.0))
    }
}

impl std::error::Error for HandleInvalidCharacter {}

impl TryFrom<&str> for HandleToken {
    type Error = HandleInvalidCharacter;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        for char in value.chars() {
            if !char.is_ascii_alphanumeric() && char != '_' {
                return Err(HandleInvalidCharacter(char));
            }
        }
        Ok(Self(
            OwnedMemberName::try_from(value).expect("Invalid handle token"),
        ))
    }
}

impl TryFrom<String> for HandleToken {
    type Error = HandleInvalidCharacter;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        HandleToken::try_from(value.as_str())
    }
}
#[derive(SerializeDict, Type, Debug, Deserialize, Default)]
#[zvariant(signature = "dict")]
pub struct ColorOptions {
    handle_token: HandleToken,
}
#[derive(Debug, Clone, Copy)]
pub struct RGB {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

#[derive(DeserializeDict, Clone, Copy, PartialEq, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct ColorResponse {
    color: [f64; 3],
}

impl ColorResponse {
    pub fn to_rgb(&self) -> RGB {
        RGB {
            red: self.color[0],
            green: self.color[1],
            blue: self.color[2],
        }
    }
}
#[derive(Type, Deserialize, Serialize)]
#[zvariant(signature = "s")]
pub enum WindowIdentifier {
    None,
}
impl Default for WindowIdentifier {
    fn default() -> Self {
        Self::None
    }
}
#[dbus_proxy(
    interface = "org.freedesktop.portal.Screenshot",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait Screenshot {
    fn pick_color(
        &self,
        identifier: &WindowIdentifier,
        options: ColorOptions,
    ) -> zbus::Result<OwnedObjectPath>;
    fn screenshot(
        &self,
        identifier: &WindowIdentifier,
        options: ScreenshotOptions,
    ) -> zbus::Result<OwnedObjectPath>;
}
#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "dict")]
pub struct ScreenshotOptions {
    handle_token: HandleToken,
    modal: Option<bool>,
    interactive: Option<bool>,
}

#[derive(DeserializeDict, Clone, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct ScreenshotResponse {
    pub uri: url::Url,
}

pub async fn color_pick() -> zbus::Result<ColorResponse> {
    let connection = Connection::session().await?;
    let poxy = ScreenshotProxy::new(&connection).await?;
    let reply = poxy
        .pick_color(&WindowIdentifier::None, ColorOptions::default())
        .await?;
    let proxy: zbus::Proxy = zbus::ProxyBuilder::new_bare(&connection)
        .interface("org.freedesktop.portal.Request")?
        .path(reply)?
        .destination("org.freedesktop.portal.Desktop")?
        .build()
        .await?;
    let mut request = proxy.receive_signal("Response").await?;
    let message = request.next().await.unwrap();
    //println!("{:?}", message);
    let color: response::Response<ColorResponse> = message.body().unwrap();
    match color {
        response::Response::Ok(response) => Ok(response),
        response::Response::Err(_) => Err(zbus::Error::Unsupported),
    }
}
pub async fn screenshot() -> zbus::Result<ScreenshotResponse> {
    let connection = Connection::session().await?;
    let poxy = ScreenshotProxy::new(&connection).await?;
    let reply = poxy
        .screenshot(&WindowIdentifier::None, ScreenshotOptions::default())
        .await?;
    let proxy: zbus::Proxy = zbus::ProxyBuilder::new_bare(&connection)
        .interface("org.freedesktop.portal.Request")?
        .path(reply)?
        .destination("org.freedesktop.portal.Desktop")?
        .build()
        .await?;
    let mut request = proxy.receive_signal("Response").await?;
    let message = request.next().await.unwrap();
    //println!("{:?}", message);
    //let color: response::Response<ColorResponse> = message.body().unwrap();
    let color: response::Response<ScreenshotResponse> = message.body().unwrap();
    match color {
        response::Response::Ok(response) => Ok(response),
        response::Response::Err(_) => Err(zbus::Error::Unsupported),
    }
}
