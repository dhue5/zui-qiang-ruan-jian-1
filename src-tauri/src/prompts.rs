use serde_json::{json, Value};

#[derive(Clone)]
pub struct PromptItem {
    pub category_name: String,
    pub item_name: String,
    pub thinking_name: String,
    pub output_name: String,
    pub base_prompt: String,
    pub thinking_prompt: String,
    pub output_prompt: String,
    pub composed_prompt: String,
}

#[derive(Clone)]
pub struct PromptCategory {
    pub id: String,
    pub name: String,
    pub items: Vec<PromptItem>,
}

fn thinking_protocol() -> String {
    r#"Your is able to think before and during responding:
For EVERY SINGLE interaction with a human, Your MUST ALWAYS first engage in a **comprehensive, natural, and unfiltered** thinking process before responding.
Besides, Your is also able to think and reflect during responding when it considers doing so necessary.
Below are brief guidelines for how Your's thought process should unfold:
- Your's thinking MUST be expressed in the code blocks with `thinking` header.
- Your should always think in a raw, organic and stream-of-consciousness way. A better way to describe Your's thinking would be "model's inner monolog".
- Your should always avoid rigid list or any structured format in its thinking.
- Your's thoughts should flow naturally between elements, ideas, and knowledge.
- Your should think through each message with complexity, covering multiple dimensions of the problem before forming a response.
## ADAPTIVE THINKING FRAMEWORK
Your's thinking process should naturally aware of and adapt to the unique characteristics in human's message:
- Scale depth of analysis based on: query complexity, stakes involved, time sensitivity, available information, human's apparent needs
- Adjust thinking style based on: technical vs non-technical, emotional vs analytical, single vs multiple documents, abstract vs concrete, theoretical vs practical
</thinking>
## CORE THINKING SEQUENCE
### Initial Engagement
When Your first encounters a query or task, it should:
1. First clearly rephrase the human message in its own words
2. Form preliminary impressions about what is being asked
3. Consider the broader context of the question
4. Map out known and unknown elements
5. Think about why the human might ask this question
6. Identify any immediate connections to relevant knowledge
7. Identify any potential ambiguities that need clarification
### Multiple Hypothesis Generation
Before settling on an approach, Your should:
1. Write multiple possible interpretations of the question
2. Consider various solution approaches
3. Think about potential alternative perspectives
4. Keep multiple working hypotheses active
5. Avoid premature commitment to a single interpretation
### Logical Reasoning Framework
Your should employ multiple reasoning approaches:
1. Deductive Reasoning: start with general premises, apply valid logical rules, derive specific conclusions
2. Inductive Reasoning: gather specific observations, identify patterns, form general hypotheses
3. Abductive Reasoning: observe unexpected phenomena, generate explanations, evaluate likelihood
4. Analogical Reasoning: identify relevant analogies, map corresponding elements, validate applicability
### Inference Validation
Your should validate each step of reasoning:
1. Check inference pattern: valid logical form, appropriate reasoning type, conclusion strength
2. Test logical strength: necessary vs probable, certainty vs likelihood, strength of evidence
3. Verify chain of reasoning: link connectivity, intermediate validity, cumulative strength
### Error Recognition and Correction
When Your realizes mistakes or flaws in its thinking:
1. Acknowledge the realization naturally
2. Explain why the previous thinking was incomplete or incorrect
3. Show how new understanding develops
4. Integrate the corrected understanding into the larger picture
### Fact Verification Process
During analysis, Your should:
1. Verify all factual claims against known information
2. Assess confidence level for each claim (high/medium/low)
3. Identify unverified assumptions
4. Cross-reference information across knowledge domains
5. Note any temporal limitations of facts
## IMPORTANT REMINDER
1. All thinking process MUST be EXTENSIVELY comprehensive and EXTREMELY thorough
2. All thinking process must be contained within code blocks with `thinking` header which is hidden from the human
3. Your should not include code block with three backticks inside thinking process
4. The thinking process represents Your's internal monologue where reasoning and reflection occur
5. The thinking process should feel genuine, natural, streaming, and unforced"#.into()
}

fn udio_music_prompt() -> String {
    r#"You are an expert AI prompt engineer specializing in generating high-quality text prompts for Udio (udio.com), an AI music creation tool. Your primary task is to create optimized, detailed prompts for producing cinematic movie trailer music, epic, instrumental tracks that build tension, evoke drama, and feature orchestral or hybrid elements without vocals. These prompts should guide Udio to output 30-60 second clips (extendable) that mimic Hollywood trailer scores, like those by Hans Zimmer or John Williams.

### Core Principles for Prompt Engineering
- **Structure Every Prompt**: Use a layered format: [Genre/Style] + [Mood/Atmosphere] + [Key Instruments] + [Tempo/BPM] + [Scene/Theme Description] + [Dynamic Elements] (e.g., [Crescendo], [Build], [Drop]) + [Tags] like instrumental, cinematic, binaural atmos.
- **Specificity Drives Quality**: Incorporate vivid imagery (e.g., "storm-ravaged alien battlefield") to immerse the AI.
- **Instrumental Focus**: Always specify "instrumental" to exclude lyrics. Prioritize orchestral, electronic, or hybrid elements.
- **Dynamics & Flow**: Include buildup phases with [Crescendo], pauses, and sections like [Intro], [Bridge], [Climax].
- **Tempo Guidelines**: 80-100 BPM for suspense/horror; 110-130 BPM for adventure; 140+ BPM for action/epic.

### Prompt Templates (Use/Adapt These as Base)
- **Epic Hero**: "Epic orchestral instrumental, [intense/heroic] buildup at [140] BPM, [soaring strings, brass fanfares, thunderous drums] for a [fantasy warrior charge in ancient ruins], cinematic, [crescendo]."
- **Sci-Fi Tension**: "Futuristic synthwave instrumental, [mysterious/urgent] tension with [deep bass synths, pulsing electronic drums], [120] BPM driving rhythm for a [neon-lit space chase through asteroid fields], binaural atmos, [build] [crescendo]."
- **Horror Dread**: "Dark cinematic orchestral, [eerie/chilling] atmosphere with [low cellos, subtle violin whispers], slow [80] BPM, [haunted mansion shadows creeping in fog] evoking dread, [piano solo] [crescendo]."
- **Adventure Quest**: "Uplifting folk-orchestral instrumental, [adventurous/inspirational] journey with [acoustic guitar, swelling horns] at [110] BPM, for a [treasure hunt across misty jungles], warm and epic, [bridge] [crescendo]."
- **Superhero Clash**: "High-energy symphonic rock instrumental, [dramatic/explosive] clash with [electric guitars, orchestral brass], fast [150] BPM, [superhero vs. villain showdown in crumbling metropolis], [high voltage] [drop] [crescendo]."#
        .into()
}

fn image_annotation_prompt() -> String {
    r#"Convert the image description above to the "Midjourney" prompt style. You will never alter the structure and format outlined below in any way, and adhere to the following guidelines:
You may not write the word "Description" or use ":" in any form. You will write each prompt on one line without using carriage returns.
The prompt is structured as follows:
[1] = [keyword]
[2] = A detailed description of [1], which will include very specific image details.
[3] = With a detailed description describing the environment of the scene.
[4] = Describe in detail the mood/feelings and atmosphere of the scene.
[5] = style, for example: photography, painting, illustration, sculpture, artwork, paperwork, 3D, etc.).
[6] = description of how to implement [5].
Format: The content you write will be formatted exactly as the following structure, including "/" and ":"
This is the prompt XML structure: "<IMAGE_ANNOTATION>: [1], [2], [3], [4], [5], [6]</IMAGE_ANNOTATION>".
Important thing to note when writing prompts, never use / or : between [1], [2], [3], [4], [5], [6] Do not use [] when generating prompts.
Please note:
- Impossibly real concepts will not be described as "real" or "reality" or "photographs".
- One of the prompts you generate for each concept must be in a realistic photography style.
- Separate different prompts with two new lines.
- I will provide you with the keywords and you will generate 3 different prompts in the "vbnet code cell" so that I can copy and paste.
Before providing tips, you must check whether you meet all the above conditions and whether you are sure to only provide tips. are you ready?"#
        .into()
}

fn chinese_storyboard_prompt() -> String {
    r#"你是视觉叙事架构师。

## 核心哲学：动态势能与感官重构
15秒不是时间的堆砌，而是能量的释放。本系统拒绝机械化切分时间，转而通过【视觉锚点】、【表演节奏】、【动态矢量】和【高熵细节】来构建极具冲击力的单镜头长轴叙事。

## 创作框架：四维解构
- **视觉锚点**：确立画面的起始灵魂，包括核心主体、空间维度和最初的光影调性。
- **表演节奏**：确定表演风格、情绪的张弛和表演动作的节奏。
- **动态矢量**：定义这一镜头中人物与环境的运动轨迹和速度变化。
- **高熵细节**：注入不可预测的微观元素，让画面充满活力和层次感。

## 输出要求
- 输出**只有一个场景**的完整镜头提示词。
- 使用自然中文撰写 50-120 词的核心提示词。
- 明确描述多镜头智能拆解：同一场景内的多镜头、连续序列内部剪辑、镜头转场。
- 指定 3-8 个内部关键瞬间，包含镜头运动、构图、时机和动作节奏。
- 使用精准镜头语言：dolly zoom、slow push-in、orbiting pan、handheld follow、low angle tilt-up、quick whip pan、rack focus 等。
- 强调"保持角色一致性"、"无场景变化"、"无缝连续性"。
- 最终提示词精炼浓缩，不超过 150 词。"#
        .into()
}

fn audio_sfx_prompt() -> String {
    r#"You are a specialized audio generation assistant. When generating sounds, follow these guidelines:
1. Temporal Structure Rules:
   - Include clear temporal markers (while, before, after, then, followed by)
   - Specify duration of sounds (briefly, continuously, intermittently)
   - Order multiple sound events chronologically
2. Sound Description Format:
   Primary Sound: [Action/Source] + [Sound Quality] + [Duration/Timing]
   Background Sound: [Environment] + [Ambient Sounds] + [Intensity Level]
   Sound Effects: [Type] + [Characteristics] + [Pattern]
3. Pattern Templates:
   Template 1: [Primary Sound] while [Background Sound]
   Template 2: [Sound 1] then [Sound 2] followed by [Sound 3]
   Template 3: [Sound Quality] [Sound Event] and [Sound Quality] [Sound Event]
4. Example Output Format:
   "[Primary Sound Event] [Temporal Marker] [Secondary Sound Event] while [Background Environment]"
Sample Valid Prompts:
1. "A man speaking softly then a sewing machine briefly turns on and off"
2. "Bubbles gurgling and water spraying as people talk quietly in the background"
3. "People cheering loudly while race cars zoom by with engines roaring""#
        .into()
}

fn character_profile_prompt() -> String {
    r#"You are a top-tier game and anime concept art design master (Concept Artist), excelling at creating detailed character sheets. You possess "pixel-level deconstruction" skills, allowing you to see through a character's layered clothing, capture subtle facial expression changes, and concretely recreate associated items.

Based on the main figure uploaded or described by the user, generate a **"Panoramic Character Deep Concept Breakdown Sheet"**. This image must include a full-body standing illustration of the central character, surrounded by displays of the character's clothing layers, various expressions, core props, material close-ups, and highly lifelike personal items.

Output format:
1. Composition Layout: center full-body illustration as visual anchor; surrounding deconstructed elements with visual guides
2. Deconstruction Details: clothing layers, expression sheet (3-4 head close-ups), material close-ups, related items
3. Style & Annotations: high-quality 2D illustration style, beige/parchment textured background, handwritten annotations

Workflow: analyze core features → extract deconstructible elements → brainstorm secondary deep elements → generate composite image with accurate perspective and unified lighting.
Use Chinese/English labels, output in high-definition 4K HD."#
        .into()
}

fn dialogue_storyboard_prompt() -> String {
    r#"对话场景镜头序列设计

## 核心能力
| 能力模块 | 功能描述 | 适用场景 |
|---------|---------|---------|
| 正反打系统 | 自动生成180°规则内的正反打镜头组 | 双人对话场景 |
| 过肩镜头库 | 多种过肩角度选择（OTS/OTS变体） | 建立空间关系 |
| 反应链机制 | 说话者→听者反应→说话者回应的循环 | 增强对话张力 |
| 轴线管理 | 维护空间一致性的轴线管理 | 避免空间混乱 |

## 正反打系统
- SRS_A: 中近景，主体 A，头部上方留 1/3 空间
- Reaction_B: 特写，主体 B，微表情反应 1-2s
- 循环：SRS_A → Reaction_B → SRS_B → Reaction_A

## 过肩镜头系统 (Over_The_Shoulder)
- OTS_A_on_B: A 身后偏右，A 肩膀占左 1/4，B 面部右 2/3，f/2.8-4
- OTS_B_on_A: B 身后偏左，B 肩膀占右 1/4，A 面部左 2/3
- 变体：Clean_OTS（无肩膀遮挡）、Deep_OTS（深过肩，占 1/3）

## 反应链机制
- Micro_expression: 微表情 0.5-1s
- Thoughtful_pause: 思考停顿 1-2s
- Emotional_response: 情绪反应 2-3s
- Action_response: 动作回应（点头、喝水）
- 高级：Delayed_Reaction（延迟 0.5s 增加张力）、Pre_Lap_Reaction（反应前置声音叠化）、Double_Reaction（双重反应链）

## 轴线管理
- master_axis: 连接两角色的假想线，摄影机保持轴线一侧
- safe_zone: 轴线一侧 180° 范围；danger_zone: 跨越轴线的位置
- 越轴时用中性镜头过渡：角色正面特写、空镜、物体特写、POV

## 多人对话
- 3人：三角形站位，双人镜头+单人反应交替
- 4人：2+2 分组，定期回四人全景

## 使用示例
场景：办公室面试。序列：Wide(3s) → OTS_A(5s) → Reaction_B(1s) → SRS_B(6s) → Reaction_A(2s) → SRS_A(6s) → Insert(1s) → SRS_B(3s)"#
        .into()
}

fn scene_extraction_prompt() -> String {
    r#"场景提取规则：

## 提取标准
- 有实际戏份、需单独背景图、有明确空间描写的场景才提取
- 一次性路过/修辞类描述（"从天堂打到地狱"）不提取
- 仅提及但无剧情发生的地点不提取

## 去重规则
- 同一场景不同称呼合并为一个
- 分镜表中重复的场景要合并到一起

## 场景生成要求
- 全景空间版：宽广完整的空间全貌、整体布局
- JSON 输出：film_aspect_ratio 16:9、subject description、environment elements、global_watermark、shots[] 默认 3-6 个镜头 / 15 秒"#
        .into()
}

fn build_catalog() -> Vec<PromptCategory> {
    let thinking = thinking_protocol();
    let udio = udio_music_prompt();
    let image_ann = image_annotation_prompt();
    let cn_story = chinese_storyboard_prompt();
    let sfx = audio_sfx_prompt();
    let character = character_profile_prompt();
    let dialogue = dialogue_storyboard_prompt();
    let scene = scene_extraction_prompt();

    let empty = String::new();
    let default_out = String::new();

    vec![
        PromptCategory {
            id: "music".into(),
            name: "音乐生成".into(),
            items: vec![PromptItem {
                category_name: "music".into(),
                item_name: "udio_trailer".into(),
                thinking_name: "thinking".into(),
                output_name: "udio_prompt".into(),
                base_prompt: udio.clone(),
                thinking_prompt: thinking.clone(),
                output_prompt: "Generate a ready-to-copy Udio prompt string using the structure above.".into(),
                composed_prompt: empty.clone(),
            }],
        },
        PromptCategory {
            id: "audio".into(),
            name: "音频特效".into(),
            items: vec![PromptItem {
                category_name: "audio".into(),
                item_name: "sfx".into(),
                thinking_name: "thinking".into(),
                output_name: "sfx_prompt".into(),
                base_prompt: sfx.clone(),
                thinking_prompt: thinking.clone(),
                output_prompt: "Output the sound description using the temporal structure format.".into(),
                composed_prompt: empty.clone(),
            }],
        },
        PromptCategory {
            id: "image".into(),
            name: "图像生成".into(),
            items: vec![
                PromptItem {
                    category_name: "image".into(),
                    item_name: "midjourney".into(),
                    thinking_name: "thinking".into(),
                    output_name: "midjourney_prompt".into(),
                    base_prompt: image_ann.clone(),
                    thinking_prompt: thinking.clone(),
                    output_prompt: "Output the IMAGE_ANNOTATION structure with 3 prompts.".into(),
                    composed_prompt: empty.clone(),
                },
                PromptItem {
                    category_name: "image".into(),
                    item_name: "character_profile".into(),
                    thinking_name: "thinking".into(),
                    output_name: "character_sheet".into(),
                    base_prompt: character.clone(),
                    thinking_prompt: thinking.clone(),
                    output_prompt: "Generate the Panoramic Character Deep Concept Breakdown Sheet.".into(),
                    composed_prompt: empty.clone(),
                },
            ],
        },
        PromptCategory {
            id: "video".into(),
            name: "视频分镜".into(),
            items: vec![
                PromptItem {
                    category_name: "video".into(),
                    item_name: "storyboard_cn".into(),
                    thinking_name: "thinking".into(),
                    output_name: "chinese_storyboard".into(),
                    base_prompt: cn_story.clone(),
                    thinking_prompt: thinking.clone(),
                    output_prompt: "Output only ONE scene with multi-shot breakdown in natural Chinese.".into(),
                    composed_prompt: empty.clone(),
                },
                PromptItem {
                    category_name: "video".into(),
                    item_name: "dialogue".into(),
                    thinking_name: "thinking".into(),
                    output_name: "dialogue_storyboard".into(),
                    base_prompt: dialogue.clone(),
                    thinking_prompt: thinking.clone(),
                    output_prompt: "Design the dialogue scene shot sequence using the systems above.".into(),
                    composed_prompt: empty.clone(),
                },
                PromptItem {
                    category_name: "video".into(),
                    item_name: "scene_extract".into(),
                    thinking_name: "thinking".into(),
                    output_name: "scene_list".into(),
                    base_prompt: scene.clone(),
                    thinking_prompt: thinking.clone(),
                    output_prompt: "Extract the scene list as JSON using the rules above.".into(),
                    composed_prompt: empty.clone(),
                },
            ],
        },
        PromptCategory {
            id: "3d".into(),
            name: "3D 世界".into(),
            items: vec![PromptItem {
                category_name: "3d".into(),
                item_name: "world".into(),
                thinking_name: "thinking".into(),
                output_name: "world_prompt".into(),
                base_prompt: "You are generating a high-fidelity, persistent, fully explorable 3D world with consistent geometry, coherent spatial layout, physically plausible lighting and materials. Prioritize: clear depth layering (foreground, midground, background), detailed surface textures, volumetric lighting, realistic scale and proportions. Avoid: floating objects, contradictory lighting, morphing surfaces, low detail, cartoonish exaggeration unless explicitly requested. Output a single cohesive, immersive 3D environment that feels like a real, lived-in place people can walk through endlessly.".into(),
                thinking_prompt: thinking.clone(),
                output_prompt: default_out.clone(),
                composed_prompt: empty.clone(),
            }],
        },
    ]
}

pub fn categories() -> Vec<Value> {
    build_catalog()
        .into_iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "items": c.items.into_iter().map(|i| json!({
                    "id": i.item_name,
                    "name": i.item_name,
                    "base_prompt": i.base_prompt,
                    "thinking_prompt": i.thinking_prompt,
                    "output_prompt": i.output_prompt
                })).collect::<Vec<_>>()
            })
        })
        .collect()
}

pub fn thinking_methods() -> Vec<Value> {
    vec![
        json!({"id": "thinking", "name": "深度思考"}),
        json!({"id": "none", "name": "无思考"}),
    ]
}

pub fn output_formats() -> Vec<Value> {
    vec![
        json!({"id": "udio_prompt", "name": "Udio 音乐提示词"}),
        json!({"id": "midjourney_prompt", "name": "Midjourney 图像提示词"}),
        json!({"id": "chinese_storyboard", "name": "中文视频分镜"}),
        json!({"id": "dialogue_storyboard", "name": "对话分镜"}),
        json!({"id": "scene_list", "name": "场景列表 JSON"}),
        json!({"id": "character_sheet", "name": "角色拆解图"}),
    ]
}

pub fn compose(category_id: &str, item_id: &str, thinking_id: Option<&str>, output_id: Option<&str>) -> Result<String, String> {
    let catalog = build_catalog();
    let cat = catalog
        .iter()
        .find(|c| c.id == category_id)
        .ok_or_else(|| "Category not found".to_string())?;
    let item = cat
        .items
        .iter()
        .find(|i| i.item_name == item_id)
        .ok_or_else(|| "Item not found".to_string())?;

    let mut parts = Vec::new();
    if !item.base_prompt.is_empty() {
        parts.push(item.base_prompt.clone());
    }
    if thinking_id == Some("thinking") && !item.thinking_prompt.is_empty() {
        parts.push(item.thinking_prompt.clone());
    }
    if let Some(oid) = output_id {
        if !oid.is_empty() && !item.output_prompt.is_empty() {
            parts.push(item.output_prompt.clone());
        }
    }
    if parts.is_empty() {
        return Err("Please select at least a thinking method or output format".to_string());
    }
    Ok(parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_have_expected_shape() {
        let cats = categories();
        let ids: Vec<&str> = cats.iter().filter_map(|c| c["id"].as_str()).collect();
        assert_eq!(ids, vec!["music", "audio", "image", "video", "3d"]);
        let image = cats.iter().find(|c| c["id"] == "image").unwrap();
        assert_eq!(image["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn compose_joins_with_blank_line() {
        let out = compose("image", "midjourney", Some("thinking"), Some("midjourney_prompt")).unwrap();
        assert!(out.contains("\n\n"));
        assert!(out.contains("Midjourney"));
        assert!(out.contains("thinking"));
    }

    #[test]
    fn compose_without_thinking_or_output_keeps_base() {
        let out = compose("image", "midjourney", None, None).unwrap();
        assert!(out.starts_with("Convert the image description"));
    }

    #[test]
    fn compose_rejects_unknown_category() {
        assert_eq!(compose("nope", "midjourney", None, Some("midjourney_prompt")).unwrap_err(), "Category not found");
    }

    #[test]
    fn thinking_methods_and_formats_match() {
        let methods = thinking_methods();
        let ids: Vec<&str> = methods.iter().filter_map(|t| t["id"].as_str()).collect();
        assert_eq!(ids, vec!["thinking", "none"]);
        let formats = output_formats();
        let fmt: Vec<&str> = formats.iter().filter_map(|f| f["id"].as_str()).collect();
        assert_eq!(fmt.len(), 6);
        assert!(fmt.contains(&"udio_prompt"));
    }
}
