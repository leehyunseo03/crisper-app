import React, { useEffect, useState, useRef } from 'react';
import ForceGraph2D, { ForceGraphMethods } from 'react-force-graph-2d';
import { invoke } from '@tauri-apps/api/core'; // Tauri v2 기준

interface GraphNode {
  id: string;
  group: string;
  label: string;
  val: number;
}

interface GraphLink {
  source: string | GraphNode; // force-graph가 내부적으로 객체로 치환하므로 타입 유연성 필요
  target: string | GraphNode;
}

interface GraphData {
  nodes: GraphNode[];
  links: GraphLink[];
}

const GraphVisualizer = ({ refreshKey }: { refreshKey: number }) => {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });
 const containerRef = useRef<HTMLDivElement>(null);
  const fgRef = useRef<any>(null);

  useEffect(() => {
    if (containerRef.current) {
      setDimensions({
        width: containerRef.current.clientWidth,
        height: 500, // 높이는 고정 혹은 부모에 맞춤
      });
    }
  }, [refreshKey]);

  useEffect(() => {
    console.log("🔄 Fetching graph data...");
    invoke<GraphData>('fetch_graph_data')
      .then((graphData) => {
        // 데이터가 비어있으면 로그 출력
        if (graphData.nodes.length === 0) {
            console.warn("⚠️ No nodes found in DB.");
        }
        
        // react-force-graph는 객체를 직접 수정하므로, 
        // 이전 상태와 참조가 끊긴 새로운 객체를 넣어주는 것이 안전함
        const safeData = {
            nodes: graphData.nodes.map(n => ({...n})),
            links: graphData.links.map(l => ({...l}))
        };
        
        console.log(`✅ Loaded: ${safeData.nodes.length} nodes, ${safeData.links.length} links`);
        setData(safeData);
      })
      .catch((err) => console.error("❌ Graph Load Error:", err));
  }, [refreshKey]);

  return (
    <div 
      ref={containerRef} 
      id="graph-container" 
      style={{ 
        width: '100%', 
        border: '1px solid #313244', 
        borderRadius: '8px', 
        overflow: 'hidden',
        backgroundColor: '#11111b' 
      }}
    >
      {data.nodes.length > 0 ? (
        <ForceGraph2D
          ref={fgRef}
          width={dimensions.width}
          height={dimensions.height}
          graphData={data}
          backgroundColor="#11111b"
          
          // 노드 스타일링
          nodeLabel="label"
          nodeColor={(node: any) => {
            if (node.group === 'event') return '#f38ba8';   // Red
            if (node.group === 'document') return '#89b4fa'; // Blue
            return '#a6e3a1';                                // Green (Chunk)
          }}
          nodeVal={(node: any) => node.val}
          
          // 링크 스타일링
          linkColor={() => '#45475a'}
          linkWidth={1}
          linkDirectionalParticles={2}
          linkDirectionalParticleWidth={2}
          linkDirectionalParticleSpeed={0.005}

          // 초기 줌 설정
          cooldownTicks={100}
          onEngineStop={() => fgRef.current?.zoomToFit(400)}
        />
      ) : (
        <div style={{ padding: "20px", textAlign: "center", color: "#6c7086" }}>
          데이터가 없습니다. (Step 2를 먼저 실행해주세요)
        </div>
      )}
    </div>
  );
};

export default GraphVisualizer;