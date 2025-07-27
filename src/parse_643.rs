use crate::parse_handler::ParseHandler;
use std::collections::HashMap;

pub struct Parse643;

impl ParseHandler for Parse643 {
    fn is_match(&self, path: &str) -> bool {
        path.contains("ActualControl/SuspectedActualControl")
    }

    fn parse(&self, params: HashMap<String, String>) -> (Vec<String>, String) {
        // 使用 clone 方法复制字符串，避免从引用中移动值
        let key_word = params.get("keyWord").cloned().unwrap();
        (
            vec![],
            format!("{}.json", key_word),
        )
    }
}
