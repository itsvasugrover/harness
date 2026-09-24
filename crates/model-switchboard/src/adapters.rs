//! Provider adapters. One file per API shape; all emit
//! `work_engine::processor::StreamEvent`. OpenAI-compatible first.
//! ("openai" names the wire protocol every provider here speaks —
//! GLM, DeepSeek, Muse Spark included — never a hardcoded vendor.)
pub mod openai;
