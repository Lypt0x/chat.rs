#![feature(async_closure)]

extern crate anyhow;

mod personality;
mod constants;

pub mod types;
pub mod g4f;
pub mod chimera;
pub mod huggingface;

use crate::types::Generator;

use std::sync::Arc;

use futures::future;

use once_cell::sync::Lazy;

static GENERATORS: Lazy<Vec<Arc<dyn Generator + Send + Sync>>> =
  Lazy::new(|| {
    vec![ Arc::new( g4f::chatgptai::ChatgptAiGenerator )
        , Arc::new( g4f::llama2::Llama2Generator )
        , Arc::new( g4f::gptforlove::GptForLoveGenerator )
        , Arc::new( huggingface::HuggingFaceGenerator )
        ]
  });

pub async fn generate(msg: &str, bot_name: &str, fancy: bool, system_context: &str) -> anyhow::Result<String> {
  let fmode =
    if fancy {
      ! (msg.contains("please")
      || msg.contains("пожалуйста")
      || msg.contains("Please")
      || msg.contains("Пожалуйста")
      || msg.contains("PLEASE"))
    } else {
      false
    };

  for gen in &*GENERATORS {
    if let Ok(result) = gen.call(msg, fmode, bot_name, system_context).await {
      if !result.contains("502: Bad gateway") {
        return Ok(result);
      }
    }
  }

  Err( anyhow::anyhow!("All generators failed") )
}

pub async fn generate_all<'a>(msg: &str, bot_name: &str, fancy: bool, system_context: &str)
                                -> Vec<(&'a str, anyhow::Result<String>)> {
  let fmode =
    if fancy {
      ! (msg.contains("please")
      || msg.contains("пожалуйста")
      || msg.contains("Please")
      || msg.contains("Пожалуйста")
      || msg.contains("PLEASE"))
    } else {
      false
    };

  let genz = (&*GENERATORS).into_iter().map(
    |gen| async move { ( gen.name(),
      if gen.enabled_for_multigen()
             { gen.call(msg, fmode, bot_name, system_context).await }
        else { anyhow::Ok(String::from("disabled")) } )
    }
  );

  future::join_all(genz).await
}

pub async fn chat(msg: &str, bot_name: &str, system_context: &str) -> anyhow::Result<String> {
  generate(msg, bot_name, true, system_context).await
}
