import React, { useEffect, useState, useRef } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { invoke } from '@tauri-apps/api/core';

// 🌟 Rust 데이터 구조와 일치하는 인터페이스 정의
interface GraphNode {
  id: string;
  group: string; // "event" | "document" | "entity" | "chunk"
  label: string;
  info?: string; // Rust의 Option<String>은 undefined일 수 있음
  val: number;   // 🆕 Rust에서 추가된 노드 크기 값
}

interface GraphLink {
  source: string | any;
  target: string | any;
  label?: string; // 🆕 관계명 (related_to)
}

interface GraphData {
  nodes: GraphNode[];
  links: GraphLink[];
}

interface GraphVisualizerProps {
  refreshKey: number;
  viewMode?: string;
  onNodeClick: (node: GraphNode) => void;
}

const GraphVisualizer = ({ refreshKey, viewMode = "all", onNodeClick }: GraphVisualizerProps) => {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const [hoverNode, setHoverNode] = useState<any>(null)
  const containerRef = useRef<HTMLDivElement>(null);
  const fgRef = useRef<any>(null);

  // 1. 컨테이너 크기 감지 (반응형)
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
    // viewMode 파라미터 전달 (Rust의 view_mode 인자 매핑됨)
    invoke<GraphData>('fetch_graph_data', { viewMode: viewMode }) 
      .then((graphData) => {
        // 객체 복사를 통해 상태 업데이트 (ForceGraph가 객체를 변형시키기 때문)
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
          onNodeClick={onNodeClick}
          
          // 🌟 노드 크기: Rust에서 전달받은 'val' 속성 사용
          nodeVal={node => node.val}
          
          // --- 호버 이벤트 설정 ---
          onNodeHover={(node) => setHoverNode(node)}
          
          // --- 간선(Link) 디자인 ---
          linkCanvasObjectMode={() => 'after'}
          linkCanvasObject={(link: any, ctx, globalScale) => {
            const label = link.label;
            if (!label) return;

            // 소스/타겟의 ID 또는 객체 참조 처리
            const sourceId = typeof link.source === 'object' ? link.source.id : link.source;
            const targetId = typeof link.target === 'object' ? link.target.id : link.target;
            const isConnected = hoverNode && (sourceId === hoverNode.id || targetId === hoverNode.id);

            // 줌 레벨이 낮을 때(멀리 볼 때)는 텍스트 숨김 (성능 최적화)
            if (!isConnected && globalScale < 1.5) return;

            const start = link.source;
            const end = link.target;
            // 좌표가 계산되지 않았으면 리턴
            if (typeof start !== 'object' || typeof end !== 'object') return;

            const textPos = {
              x: start.x + (end.x - start.x) * 0.5,
              y: start.y + (end.y - start.y) * 0.5,
            };

            const fontSize = isConnected ? (16 / globalScale) : (8 / globalScale);
            ctx.font = `${isConnected ? 'bold' : 'normal'} ${fontSize}px Sans-Serif`;
            
            const textWidth = ctx.measureText(label).width;
            const padding = 2;
            
            // 텍스트 배경 (가독성)
            ctx.fillStyle = isConnected ? 'rgba(249, 226, 175, 0.95)' : 'rgba(30, 30, 46, 0.8)';
            ctx.fillRect(
              textPos.x - (textWidth / 2) - padding,
              textPos.y - (fontSize / 2) - padding,
              textWidth + (padding * 2),
              fontSize + (padding * 2)
            );

            // 텍스트
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillStyle = isConnected ? '#11111b' : '#cba6f7';
            ctx.fillText(label, textPos.x, textPos.y);
          }}

          // 링크 색상
          linkColor={(link: any) => {
            if (hoverNode && (link.source.id === hoverNode.id || link.target.id === hoverNode.id)) {
              return '#f9e2af';
            }
            return '#45475a';
          }}
          
          linkWidth={(link: any) => {
            return hoverNode && (link.source.id === hoverNode.id || link.target.id === hoverNode.id) ? 2 : 1;
          }}

          linkDirectionalArrowLength={(link: any) => {
            return hoverNode && (link.source.id === hoverNode.id || link.target.id === hoverNode.id) ? 5 : 2;
          }}

          // 노드 색상: 그룹별 지정
          nodeColor={(node: any) => {
            if (node === hoverNode) return '#f38ba8';
            switch (node.group) {
              case 'document': return '#89b4fa'; // 파랑
              case 'entity': return '#fab387';   // 주황
              case 'chunk': return '#45475a';    // 회색
              default: return '#a6adc8';
            }
          }}
        />
      )}
      
      {data.nodes.length === 0 && (
        <div style={{ position: "absolute", top: "50%", left: "50%", transform: "translate(-50%, -50%)", color: "#45475a", pointerEvents: "none", textAlign: "center" }}>
          <h3>데이터가 없습니다</h3>
          <p>우측 패널에서 PDF 폴더를 선택하고 분석을 시작하세요.</p>
        </div>
      )}
    </div>
  );
};

export default GraphVisualizer;