use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareTextOptions {
  pub mime_type: Option<String>,
}

#[derive(Serialize)]
pub struct ShareTextPayload {
  pub text: String,
  #[serde(flatten)]
  pub options: ShareTextOptions,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareFileOptions {
  pub mime_type: Option<String>,
  pub title: Option<String>,
}

#[derive(Serialize)]
pub struct ShareFilePayload {
  pub url: String,
  #[serde(flatten)]
  pub options: ShareFileOptions,
}
