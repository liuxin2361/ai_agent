// fn main() {
//     // // println!("Hello, world!");
//     // let name = "DeepSeek";
//     // // name = "DeepSeek AI";
//     // let mut count = 0;
//     // count += 1;

//     // println!("name: {}, count: {}", name, count);

//     // let a = 1;                      // i32，栈上，复制值
//     // let b = "hello";                // &str，指向只读内存，复制引用
//     // let c = String::from("hello");  // String，堆上，转移所有权

//     // let s1 = "hello"; // 类型是 &str（字符串切片引用）
//     // let s1 = String::from("hello"); // 类型是 String（堆分配的字符串）
//     // let s2 = s1;
//     // println!("{}", s1);
// }
use serde::{Deserialize, Serialize}; // 引入序列化/反序列化
use std::io::{self, Write}; // 引入输入输出

// 数据结构定义
// 对话消息结构体，对应 API 的 messages 数组中的每一条
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,    // 角色：system、user、assistant
    content: String, // 消息内容
}

#[derive(Serialize)]
struct ChatRequest {
    model: String, // 模型名称
    messages: Vec<Message>, // 对话消息列表 Vec 相当于 Java 的 ArrayList / Python 的 list
                   // temperature: f32,       // 生成文本的随机程度
                   // max_tokens: u32,        // 生成文本的最大长度
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>, // API 返回的选项列表
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message, // 每个选项包含一个消息
}

// 核心函数
// & 表示引用，意思是"借用这个值，但不拥有它"
async fn chat(
    client: &reqwest::Client,
    api_key: &str,
    messages: &Vec<Message>,
) -> Result<String, Box<dyn std::error::Error>> {
    // 创建 HTTP 客户端
    let request = ChatRequest {
        model: String::from("deepseek-v4-flash"), // 模型名称
        messages: messages.clone(),               // 克隆消息列表
                                                  // temperature: 0.7,                         // 随机程度
                                                  // max_tokens: 150,                          // 最大长度
    };

    let response = client
        .post("https://api.deepseek.com/v1/chat/completions") // API 端点
        .header("Authorization", format!("Bearer {}", api_key)) // 添加授权头
        .header("Content-Type", "application/json")
        .json(&request) // 将请求体序列化为 JSON
        .send() // 发送请求
        .await?
        .json::<ChatResponse>() // 解析响应为 ChatResponse 结构体
        .await?;

    // 取出回复内容
    let reply = response.choices[0].message.content.clone();
    Ok(reply)
}

#[tokio::main] // 宏启动异步运行时，固定写法
async fn main() {
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("请设置环境变量 DEEPSEEK_API_KEY");
    let client = reqwest::Client::new(); // 创建 HTTP 客户端
    let mut messages: Vec<Message> = Vec::new(); // 创建一个空的消息列表

    println!("=== DeepSeek 命令行对话 ===");
    println!("输入 exit 退出\n");

    loop {
        // 相当于 while True
        print!("你: ");
        io::stdout().flush().unwrap(); // 确保立即显示

        // 读取用户输入
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim(); // 去掉换行符，相当于 Python 的 strip()

        if input.eq_ignore_ascii_case("exit") {
            println!("退出对话，感谢使用！");
            break; // 退出循环
        }

        if input.is_empty() {
            continue; // 如果输入为空，继续下一轮循环
        }

        messages.push(Message {
            role: String::from("user"),
            content: input.to_string(),
        });

        // 调用 API
        print!("AI: ");
        io::stdout().flush().unwrap();

        //  Rust 的模式匹配，相当于增强版的 switch，但比 switch 强大很多
        match chat(&client, &api_key, &messages).await {
            Ok(reply) => {
                println!("{}", reply);
                messages.push(Message {
                    role: String::from("assistant"),
                    content: reply,
                });
            }
            Err(e) => {
                eprintln!("请求失败: {}\n", e);
            }
        }
    }
}
