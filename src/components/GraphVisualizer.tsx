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

interface GraphVisualizerProps {
  refreshKey: number;
  viewMode?: string;
  onNodeClick: (node: GraphNode) => void; // 👈 부모 컴포넌트로 노드 정보를 넘겨줄 콜백
}

// 🚨 [수정됨] viewMode props 추가 (Rust 백엔드 인자 대응)
const GraphVisualizer = ({ refreshKey, viewMode = "all", onNodeClick }: GraphVisualizerProps) => {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const [hoverNode, setHoverNode] = useState<any>(null)
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
          onNodeClick={onNodeClick}
          
          // --- 호버 이벤트 설정 ---
          onNodeHover={(node) => setHoverNode(node)}
          
          // --- 간선(Link) 디자인: 호버 상태에 따라 동적 렌더링 ---
          linkCanvasObjectMode={() => 'after'} // 기존 선 위에 추가로 그림
          linkCanvasObject={(link: any, ctx, globalScale) => {
            // 데이터 확인: label이 없으면 리턴
            const label = link.label;
            if (!label) return;

            // 소스/타겟이 객체인지 문자열인지 판별하여 호버 여부 확인
            const sourceId = typeof link.source === 'object' ? link.source.id : link.source;
            const targetId = typeof link.target === 'object' ? link.target.id : link.target;
            const isConnected = hoverNode && (sourceId === hoverNode.id || targetId === hoverNode.id);

            // 호버되지 않은 상태에서 줌이 너무 낮으면 렌더링 스킵
            if (!isConnected && globalScale < 1.5) return;

            // 좌표 추출
            const start = link.source;
            const end = link.target;
            if (typeof start !== 'object' || typeof end !== 'object') return;

            const textPos = {
              x: start.x + (end.x - start.x) * 0.5,
              y: start.y + (end.y - start.y) * 0.5,
            };

            // 폰트 설정: 호버 시 더 크고 굵게
            const fontSize = isConnected ? (16 / globalScale) : (8 / globalScale);
            ctx.font = `${isConnected ? 'bold' : 'normal'} ${fontSize}px Sans-Serif`;
            
            // 가독성을 위한 텍스트 배경 박스
            const textWidth = ctx.measureText(label).width;
            const padding = 2;
            
            ctx.fillStyle = isConnected ? 'rgba(249, 226, 175, 0.95)' : 'rgba(30, 30, 46, 0.8)';
            ctx.fillRect(
              textPos.x - (textWidth / 2) - padding,
              textPos.y - (fontSize / 2) - padding,
              textWidth + (padding * 2),
              fontSize + (padding * 2)
            );

            // 텍스트 그리기
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillStyle = isConnected ? '#11111b' : '#cba6f7';
            ctx.fillText(label, textPos.x, textPos.y);
          }}

          // 호버 시 간선 색상도 강조
          linkColor={(link: any) => {
            if (hoverNode && (link.source.id === hoverNode.id || link.target.id === hoverNode.id)) {
              return '#f9e2af'; // 호버 연결선은 노란색
            }
            return '#45475a';
          }}
          
          linkWidth={(link: any) => {
            return hoverNode && (link.source.id === hoverNode.id || link.target.id === hoverNode.id) ? 2 : 1;
          }}

          linkDirectionalArrowLength={(link: any) => {
            return hoverNode && (link.source.id === hoverNode.id || link.target.id === hoverNode.id) ? 5 : 2;
          }}

          nodeColor={(node: any) => {
            if (node === hoverNode) return '#f38ba8'; // 호버된 노드는 빨간색 계열
            if (node.group === 'entity') return '#fab387';
            return '#45475a';
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