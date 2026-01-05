// src/components/GraphVisualizer.tsx
import React, { useEffect, useState, useRef } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { invoke } from '@tauri-apps/api/core';

interface GraphNode {
  id: string;
  group: string;
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

const GraphVisualizer = ({ refreshKey }: { refreshKey: number }) => {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  // 초기 크기는 0으로 두고 ResizeObserver로 설정
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
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

  // 2. 데이터 로드
  useEffect(() => {
    invoke<GraphData>('fetch_graph_data')
      .then((graphData) => {
        const safeData = {
          nodes: graphData.nodes.map(n => ({...n})),
          links: graphData.links.map(l => ({...l}))
        };
        setData(safeData);
      })
      .catch((err) => console.error("Graph Load Error:", err));
  }, [refreshKey]);

  return (
    <div 
      ref={containerRef} 
      style={{ 
        width: '100%', 
        height: '100%', // 부모(Genifier Background)를 가득 채움
        overflow: 'hidden',
        backgroundColor: '#11111b' 
      }}
    >
      {/* 크기가 측정된 후에 그래프 렌더링 */}
      {dimensions.width > 0 && dimensions.height > 0 && (
        <ForceGraph2D
          ref={fgRef}
          width={dimensions.width}
          height={dimensions.height}
          graphData={data}
          backgroundColor="#11111b"
          
          nodeLabel="label"
          nodeColor={(node: any) => {
            if (node.group === 'event') return '#f38ba8';
            if (node.group === 'document') return '#89b4fa';
            return '#a6e3a1';
          }}
          nodeVal={(node: any) => node.val}
          
          linkColor={() => '#45475a'}
          linkWidth={1}
          linkDirectionalParticles={2}
          linkDirectionalParticleWidth={2}
          
          onEngineStop={() => {
            // 데이터 로드 직후 한 번만 핏하게 줌
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
          pointerEvents: "none" // 그래프 조작 방해 금지
        }}>
          데이터가 없습니다. 우측 패널에서 그래프를 생성해주세요.
        </div>
      )}
    </div>
  );
};

export default GraphVisualizer;