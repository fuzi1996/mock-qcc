use crate::parse_handler::ParseHandler;
use std::collections::HashMap;

pub struct Parse628;

impl ParseHandler for Parse628 {
    fn is_match(&self, path: &str) -> bool {
        path.contains("Beneficiary/GetBeneficiary")
    }

    fn parse(&self, params: HashMap<String, String>) -> (Vec<String>, String) {
        // 使用 clone 方法复制字符串，避免从引用中移动值
        let company_name = params.get("companyName").cloned().unwrap();
        // 使用 cloned 方法获取 Option<String>，再使用 unwrap_or 设置默认值
        let page_index = params.get("pageIndex").cloned().unwrap_or("1".to_string());
        let page_size = params.get("pageSize").cloned().unwrap_or("1".to_string());

        (
            vec![company_name],
            format!("{}_{}.json", page_index, page_size),
        )
    }
}
