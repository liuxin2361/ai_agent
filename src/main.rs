use serde::{Deserialize, Serialize}; // 引入序列化/反序列化
use std::io::{self, Write}; // 引入输入输出

// ===== 数据结构 =====
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,              // 角色：system、user、assistant
    content: String,           // 消息内容
    reasoning_content: String, // 新增：思考过程内容
}

#[derive(Serialize)]
struct ThinkingType {
    #[serde(rename = "type")]
    thinking_type: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,            // 模型名称
    messages: Vec<Message>,   // 对话消息列表
    stream: bool,             // 是否启用流式响应
    thinking: ThinkingType,   // 新增
    reasoning_effort: String, // 新增
}

#[derive(Deserialize, Debug)]
// 流式响应的数据结构
struct StreamResponse {
    choices: Vec<StreamChoice>, // API 返回的选项列表
}

#[derive(Deserialize, Debug)]
struct StreamChoice {
    delta: Delta,                  // 每个选项包含一个增量消息
    finish_reason: Option<String>, // 结束原因，可能是 "stop" 或 "length"
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
    reasoning_content: Option<String>, // 思考过程内容
}

// ===== Agent 结构体 =====
struct Agent {
    client: reqwest::Client, // HTTP 客户端
    api_key: String,
    messages: Vec<Message>, // 对话消息列表
    system_prompt: String,  // 系统提示语
    max_history: usize,     // 最大历史消息数
}

impl Agent {
    fn new(api_key: &str, system_prompt: &str, max_history: usize) -> Self {
        Agent {
            client: reqwest::Client::new(), // 创建 HTTP 客户端
            api_key: api_key.to_string(),
            messages: Vec::new(), // 初始化消息列表
            system_prompt: system_prompt.to_string(),
            max_history,
        }
    }

    // 添加消息到对话历史
    fn add_message(&mut self, role: &str, content: &str, reasoning_content: &str) {
        // &mut self ,借用并允许修改 Agent 实例
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
            reasoning_content: reasoning_content.to_string(),
        });
        // 保持消息列表不超过 max_history 条
        if self.messages.len() > self.max_history {
            self.messages.remove(0); // 移除最早的消息
        }
    }

    // 构建发送给 API 的完整消息列表（在最前面插入 system prompt）
    fn build_messages(&self) -> Vec<Message> {
        let mut full_messages = vec![Message {
            // vec![] 创建一个新的空向量，并在其中添加一个Message结构体，表示系统提示语
            role: String::from("system"),
            content: self.system_prompt.clone(),
            reasoning_content: String::new(), // 系统提示语没有思考内容
        }];
        // extend 把另一个 Vec 的元素追加进来，相当于 Python 的 list.extend()
        full_messages.extend(self.messages.clone()); // 添加用户和助手的消息
        full_messages // 返回完整的消息列表
    }

    // 流式调用 API，边生成边打印
    async fn chat_stream(&mut self, user_input: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.add_message("user", user_input, ""); // 添加用户输入到消息列表

        let request_body = ChatRequest {
            model: String::from("deepseek-v4-flash"), // 使用的模型名称
            messages: self.build_messages(),          // 构建完整的消息列表
            stream: true,                             // 启用流式响应
            thinking: ThinkingType {
                thinking_type: String::from("enabled"), // 启用思考模式
            },
            reasoning_effort: String::from("high"),
        };

        // 发送请求，获取流式响应
        let mut response = self
            .client
            .post("https://api.deepseek.com/chat/completions") // API 端点
            .header("Authorization", format!("Bearer {}", self.api_key)) // 添加授权头
            .header("Content-Type", "application/json")
            .json(&request_body) // 将请求体序列化为 JSON
            .send()
            .await?; // 发送请求并等待响应

        let mut full_response = String::new(); // 收集完整回复，用于存入历史
        let mut full_thingking = String::new(); // 收集完整思考过程
        let mut is_thinking = false; // 标记当前是否在输出思考过程

        print!("AI: "); // 打印助手提示
        io::stdout().flush()?; // 刷新输出缓冲区，确保提示立即显示

        // 逐行读取流式响应
        // chunk 是每次收到的数据块，可能包含多行数据
        while let Some(chunk) = response.chunk().await? {
            // 迭代响应中的每个数据块
            let chunk_str = String::from_utf8_lossy(&chunk); // 将字节转换为字符串

            for line in chunk_str.lines() {
                // API 的流式响应每行以 "data: " 开头，后面跟着 JSON 数据
                // 流式响应每行格式是 "data: {...}" 或 "data: [DONE]"
                if line.starts_with("data: ") {
                    let data_str = &line[6..]; // 去掉 "data: " 前缀
                    if data_str == "[DONE]" {
                        break; // 流结束
                    }

                    // 解析 JSON，如果解析失败就跳过这一行
                    if let Ok(stream_response) = serde_json::from_str::<StreamResponse>(data_str) {
                        let delta = &stream_response.choices[0].delta; // 取出增量消息
                        if let Some(reasoning) = &delta.reasoning_content {
                            if !is_thinking {
                                print!("\n💭 思考中...\n");
                                is_thinking = true;
                            }
                            print!("{}", reasoning); // 打印思考过程
                            io::stdout().flush()?; // 刷新输出缓冲区
                            full_thingking.push_str(reasoning); // 收集完整思考过程
                        }

                        // 输出正式回复
                        if let Some(content) = &delta.content {
                            if is_thinking {
                                print!("\n\n💬 回答：\n");
                                is_thinking = false;
                            }
                            print!("{}", content);
                            io::stdout().flush()?;
                            full_response.push_str(content);
                        }
                    }
                }
            }
        }

        println!("\n"); // 打印换行符，结束助手输出

        if !full_response.is_empty() {
            self.add_message("assistant", &full_response, &full_thingking); // 添加助手回复到消息列表
        }

        Ok(())
    }
}

// ===== 主函数 =====
#[tokio::main] // 宏启动异步运行时，固定写法
async fn main() {
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("请设置环境变量 DEEPSEEK_API_KEY");
    let mut agent = Agent::new(
        &api_key,
        "你是一个专业的 Rust 编程助手，回答简洁清晰，适当给出代码示例。",
        20,
    ); // 创建 Agent 实例，设置系统提示语和最大历史消息数

    println!("=== DeepSeek Agent ===");
    println!("输入 exit 退出\n");

    loop {
        print!("你: ");
        io::stdout().flush().unwrap(); // 确保立即显示

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") {
            println!("退出对话，感谢使用！");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // 使用 Agent 进行对话
        if let Err(e) = agent.chat_stream(input).await {
            eprintln!("请求失败: {}\n", e);
        }
    }
}
