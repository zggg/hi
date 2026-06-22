    use super::*;
    use crate::event::AgentEvent;
    use crate::llm::StreamChunk;

    /// 单路 forwarder 保持 provider 到达顺序。
    #[tokio::test]
    async fn stream_forwarder_preserves_chunk_order() {
        let (agent_tx, mut agent_rx) = tokio::sync::mpsc::unbounded_channel();
        let stream_tx = spawn_delta_forwarder(agent_tx, |chunk| match chunk {
            StreamChunk::Reasoning(text) => AgentEvent::ReasoningDelta { text },
            StreamChunk::Content(text) => AgentEvent::AssistantDelta { text },
        });

        for _ in 0..50 {
            stream_tx.send(StreamChunk::Reasoning("思".into())).unwrap();
        }
        stream_tx.send(StreamChunk::Content("我是".into())).unwrap();
        drop(stream_tx);

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut events = Vec::new();
        while let Ok(ev) = agent_rx.try_recv() {
            events.push(ev);
        }

        let first_content = events.iter().position(|e| {
            matches!(e, AgentEvent::AssistantDelta { .. })
        });
        let last_reasoning = events.iter().rposition(|e| {
            matches!(e, AgentEvent::ReasoningDelta { .. })
        });
        assert!(first_content.is_some());
        assert!(last_reasoning.is_some());
        assert!(first_content.unwrap() > last_reasoning.unwrap());
    }
