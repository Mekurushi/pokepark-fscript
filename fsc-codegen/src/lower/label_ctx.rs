// TODO: doesn't feel right; check if it should be restructured
pub struct LabelCtx {
    label_counter: u32,
    loop_stack: Vec<LoopContext>,
}
struct LoopContext {
    header: String,
    end: String,
}
pub struct LoopLabels {
    pub header: String,
    pub end: String,
}

impl LabelCtx {
    pub fn new() -> Self {
        Self {
            label_counter: 0,
            loop_stack: Vec::new(),
        }
    }

    pub fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}_{n}")
    }

    pub fn enter_loop(&mut self) -> LoopLabels {
        let header = self.fresh_label("loop_header");
        let end = self.fresh_label("loop_end");
        self.loop_stack.push(LoopContext {
            header: header.clone(),
            end: end.clone(),
        });
        LoopLabels { header, end }
    }
    pub fn exit_loop(&mut self) {
        self.loop_stack.pop();
    }
    pub fn loop_header(&self) -> Option<&str> {
        self.loop_stack.last().map(|l| l.header.as_str())
    }

    pub fn loop_end(&self) -> Option<&str> {
        self.loop_stack.last().map(|l| l.end.as_str())
    }
}
