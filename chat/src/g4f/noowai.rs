use crate::{
  types::Generator,
  personality::get_personality,
  constants
};

use inline_python::{ python, Context };
use std::panic::catch_unwind;

use anyhow::bail;

use std::collections::VecDeque;

use once_cell::sync::Lazy;

use tokio::sync::Mutex;

use async_trait::async_trait;

use chat_utils::help::lang;

static MSGHIST: Lazy<Mutex<VecDeque<(String, String)>>> =
  Lazy::new(|| Mutex::new( VecDeque::with_capacity(1) ));

pub struct NoowAiGenerator;

#[async_trait]
impl Generator for NoowAiGenerator {
  fn name<'a>( &self ) -> &'a str {
    "NoowAi"
  }
  async fn call( &self
               , prompt: &str
               , fmode: bool
               , personality: &str )
    -> anyhow::Result<String> {
    let mut msg_lock = MSGHIST.lock().await;
    let tmp_msg = msg_lock.as_slices();
    let russian = lang::is_russian(prompt);
    match catch_unwind(|| {
      let c = Context::new();
      c.set("prompt", prompt);
      c.set("old_messages", tmp_msg);
      c.set("is_russian", russian);
      c.set("fmode", fmode);
      c.set("PERSONALITY", get_personality(personality));
      c.run(python! {
        import sys
        import os
        import g4f
  
        result = ""
        if fmode:
          systemContext = PERSONALITY
        else:
          systemContext = "You are a helpful assistant"
        if is_russian:
          systemContext += ", you reply in Russian, you don't provide Translation"
        else:
          systemContext += ", you reply in English"
        messages = [{"role": "system", "content": systemContext}]
        if not fmode and old_messages:
          for tup in old_messages:
            if tup and len(tup) == 2:
              messages.append({"role": "user", "content": tup[0]})
              messages.append({"role": "assistant", "content": tup[1]})
        try:
          messages.append({"role": "user", "content": prompt})
          rspns = g4f.ChatCompletion.create( model=g4f.models.gpt_4, messages=messages
                                            , stream=False, auth="jwt"
                                            , provider=g4f.Provider.NoowAi )
          if not rspns:
            result = "NoowAi: Sorry, I can't generate a response right now."
            reslt = False
          else:
            reslt = True
            result = rspns
        except OSError as err:
          result = ("OS Error! {0}".format(err))
          reslt = False
        except RuntimeError as err:
          result = ("Runtime Error! {0}".format(err))
          reslt = False
      }); ( c.get::<bool>("reslt")
          , c.get::<String>("result") )
    }) {
      Ok((r,m)) => {
        if r {
          if !m.is_empty() {
            if msg_lock.len() == msg_lock.capacity() {
              msg_lock.pop_front();
            }
            if (prompt.len() + m.len()) < constants::HISTORY_LIMIT {
              msg_lock.push_back((prompt.to_string(), m.clone()));
            }
          }
          Ok(m)
        } else {
          bail!("No tokens generated: {:?}", m)
        }
      }, Err(_) => { bail!("Failed to to use NoowAi now!") }
    }
  }
}

#[cfg(test)]
mod noowai_tests {
  use super::*;
  #[tokio::test]
  async fn noowai_test() {
    let gen = NoowAiGenerator;
    let chat_response =
      gen.call("what gpt version you use?", true, "Fingon").await;
    assert!(chat_response.is_ok());
    assert!(!chat_response.unwrap().contains("is not working"));
  }
}