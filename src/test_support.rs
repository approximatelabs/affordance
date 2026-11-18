//! Shared deterministic byte tokenizer for unit and integration tests, without downloads.
use serde_json::json;
use std::{path::PathBuf, sync::OnceLock};

pub fn path() -> &'static std::path::Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let mut alphabet: Vec<_> = tokenizers::pre_tokenizers::byte_level::ByteLevel::alphabet().into_iter().collect();
        alphabet.sort_unstable();
        let vocab: serde_json::Map<String,serde_json::Value> = alphabet.into_iter().enumerate().map(|(i,c)|(c.to_string(),json!(i))).collect();
        let special=["[gMASK]","<sop>","<|system|>","<|user|>","<|assistant|>","<|observation|>","<|endoftext|>",
            "<think>","</think>","<tool_call>","</tool_call>","<arg_key>","</arg_key>","<arg_value>","</arg_value>","<tool_response>","</tool_response>",
            "<|im_start|>","<|im_end|>"];
        let added: Vec<_> = special.iter().enumerate().map(|(i,s)|json!({"id":256+i,"content":s,"single_word":false,"lstrip":false,"rstrip":false,"normalized":false,"special":true})).collect();
        let tokenizer=json!({"version":"1.0","truncation":null,"padding":null,"added_tokens":added,"normalizer":null,
            "pre_tokenizer":{"type":"ByteLevel","add_prefix_space":false,"trim_offsets":true,"use_regex":true},
            "post_processor":null,"decoder":{"type":"ByteLevel","add_prefix_space":false,"trim_offsets":true,"use_regex":true},
            "model":{"type":"BPE","dropout":null,"unk_token":null,"continuing_subword_prefix":null,"end_of_word_suffix":null,"fuse_unk":false,"byte_fallback":false,"ignore_merges":false,"vocab":vocab,"merges":[]}});
        let path=std::env::temp_dir().join(format!("aff-test-tokenizer-{}.json",std::process::id()));
        std::fs::write(&path,tokenizer.to_string()).unwrap();path
    }).as_path()
}
