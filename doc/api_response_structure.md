# 企查查接口返回值数据结构总结

```json
{
    "Status": "200",
    "Message": "【有效请求】查询成功",
    "OrderNumber": "BENEFICIARY2024051115224471920845",
    "Paging": {
        "PageSize": 10,
        "PageIndex": 1,
        "TotalRecords": 1
    },
    "Result": {
        
    }
}
```

## 固定字段
- **Status**: 响应状态码，如"200"表示成功
- **Message**: 响应消息，如"【有效请求】查询成功"
- **OrderNumber**: 订单编号，如"BENEFICIARY2024051115224471920845"

## 分页数据结构 (Paging)

如果没有分页,则该数据可不存在

```json
{
  "PageSize": 10,        // 每页记录数
  "PageIndex": 1,        // 当前页码
  "TotalRecords": 1      // 总记录数
}
```

## 结果数据 (Result)
> **注**: Result为不特定数据结构，具体内容根据接口类型变化，此处仅作标注