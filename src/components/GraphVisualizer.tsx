// src/components/GraphVisualizer.tsx
import React, { useEffect, useState, useRef } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { invoke } from '@tauri-apps/api/core';

interface GraphNode {
  id: string;
  group: string; // "event" | "document" | "entity" | "chunk"
  label: string;
  val: number;
}
interface GraphLink {
  source: string | any;
  target: string | any;
}
interface GraphData {
  nodes: GraphNode[];
  links: GraphLink[];
}

// 🚨 [수정됨] viewMode props 추가 (Rust 백엔드 인자 대응)
const GraphVisualizer = ({ refreshKey, viewMode = "all" }: { refreshKey: number, viewMode?: string }) => {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  const fgRef = useRef<any>(null);

  // 1. 컨테이너 크기 감지
  useEffect(() => {
    if (!containerRef.current) return;
    const resizeObserver = new ResizeObserver((entries) => {
      for (let entry of entries) {
        const { width, height } = entry.contentRect;
        setDimensions({ width, height });
      }
    });
    resizeObserver.observe(containerRef.current);
    return () => resizeObserver.disconnect();
  }, []);

  // 2. 데이터 로드 (Rust 통신)
  useEffect(() => {
    // 🚨 [수정됨] Rust의 fetch_graph_data(state, view_mode) 시그니처와 일치시킴
    // Rust에서 변수명은 snake_case(view_mode), JS 객체 키는 camelCase로 자동 변환될 수 있으나
    // Tauri invoke에서는 명시적으로 Rust 인자명(view_mode)을 사용하는 것이 안전함.
    invoke<GraphData>('fetch_graph_data', { viewMode: viewMode }) 
      .then((graphData) => {
        const safeData = {
          nodes: graphData.nodes.map(n => ({...n})),
          links: graphData.links.map(l => ({...l}))
        };
        setData(safeData);
      })
      .catch((err) => console.error("Graph Load Error:", err));
  }, [refreshKey, viewMode]);

  return (
    <div 
      ref={containerRef} 
      style={{ 
        width: '100%', 
        height: '100%', 
        overflow: 'hidden',
        backgroundColor: '#11111b' 
      }}
    >
      {dimensions.width > 0 && dimensions.height > 0 && (
        <ForceGraph2D
          ref={fgRef}
          width={dimensions.width}
          height={dimensions.height}
          graphData={data}
          backgroundColor="#11111b"
          
          nodeLabel="label"
          // 🚨 [수정됨] 백엔드 모델(Entity, Chunk)에 따른 색상 분기 추가
          nodeColor={(node: any) => {
            switch (node.group) {
              case 'event': return '#f38ba8';    // Red (Import Session)
              case 'document': return '#89b4fa'; // Blue (PDF Files)
              case 'entity': return '#fab387';   // Orange (Knowledge Entities) - 중요!
              case 'chunk': return '#45475a';    // Gray (Raw Text Chunks) - 배경처럼 처리
              default: return '#a6e3a1';         // Green (Default)
            }
          }}
          // 백엔드에서 val을 보내주므로 노드 크기에 반영됨
          nodeVal={(node: any) => node.val}
          
          // 링크 스타일
          linkColor={() => '#585b70'}
          linkWidth={1.5}
          linkDirectionalParticles={2}
          linkDirectionalParticleWidth={2}
          
          onEngineStop={() => {
            if(data.nodes.length > 0) fgRef.current?.zoomToFit(400);
          }}
        />
      )}
      
      {data.nodes.length === 0 && (
        <div style={{ 
          position: "absolute", 
          top: "50%", 
          left: "50%", 
          transform: "translate(-50%, -50%)", 
          color: "#45475a",
          pointerEvents: "none",
          textAlign: "center"
        }}>
          <h3>데이터가 없습니다</h3>
          <p>우측 패널에서 PDF 폴더를 선택하고 분석을 시작하세요.</p>
        </div>
      )}
    </div>
  );
};

export default GraphVisualizer;