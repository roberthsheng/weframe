// VideoEditor.js
import React, { useState, useEffect, useCallback, useRef } from 'react';
import init, { WeframeClient } from 'weframe-client';
import VideoPreview from './VideoPreview';
import _ from 'lodash';

const TRACK_HEIGHT = 50;
const PIXELS_PER_SECOND = 50;

const VideoEditor = () => {
    const [client, setClient] = useState(null);
    const [project, setProject] = useState({
        clips: [],
        duration: { secs: 300, nanos: 0 },
        collaborators: []
    });
    const [draggingClip, setDraggingClip] = useState(null);
    const [resizingClip, setResizingClip] = useState(null);
    const [currentTime, setCurrentTime] = useState(0);
    const [isPlaying, setIsPlaying] = useState(false);
    const wsRef = useRef(null);
    const timelineRef = useRef(null);
    const playIntervalRef = useRef(null);

    const updateProject = useCallback((clientInstance) => {
        try {
            const projectData = clientInstance.get_project();
            console.log('Received project data:', projectData);
            setProject(prevProject => {
                if (!_.isEqual(prevProject, projectData)) {
                    return projectData;
                }
                return prevProject;
            });
        } catch (error) {
            console.error("Failed to get project data:", error);
        }
    }, []);

    const handleMessage = useCallback((event) => {
        const operation = JSON.parse(event.data);
        console.log('Received operation:', operation);
        if (operation.operation.UpdateCollaboratorCursor) {
            const { collaborator_id, new_position } = operation.operation.UpdateCollaboratorCursor;
            setProject(prevProject => ({
                ...prevProject,
                collaborators: prevProject.collaborators.map(c =>
                    c.id === collaborator_id ? { ...c, cursor_position: new_position } : c
                )
            }));
        } else {
            updateProject(client);
        }
    }, [client, updateProject]);

    useEffect(() => {
        const initializeApp = async () => {
            try {
                await init();
                console.log("WebAssembly module initialized successfully");
                const ws = new WebSocket('ws://localhost:3030/ws/default-session');

                ws.onopen = () => {
                    console.log('WebSocket connected successfully');
                    const newClient = new WeframeClient('ws://localhost:3030/ws/default-session', 'user1', 'User 1');
                    setClient(newClient);
                    updateProject(newClient);
                };

                ws.onmessage = handleMessage;

                ws.onerror = (error) => {
                    console.error('WebSocket error:', error);
                };

                ws.onclose = (event) => {
                    console.log('WebSocket closed:', event.code, event.reason);
                };

                wsRef.current = ws;
            } catch (error) {
                console.error("Failed to initialize:", error);
            }
        };

        initializeApp();

        return () => {
            if (wsRef.current) {
                wsRef.current.close();
            }
        };
    }, []);

    const addClip = useCallback(() => {
        if (client) {
            const startTime = 0;
            const endTime = 10;
            const track = project.clips.length % 3;

            const placeholderVideo = "https://test-videos.co.uk/vids/bigbuckbunny/mp4/h264/360/Big_Buck_Bunny_360_10s_1MB.mp4";

            console.log("Adding clip:", { startTime, endTime, track, placeholderVideo });
            try {
                client.add_clip(startTime, endTime, track, placeholderVideo);
                console.log("Clip added successfully");
                updateProject(client);
            } catch (error) {
                console.error("Failed to add clip:", error);
            }
        }
    }, [client, project.clips.length, updateProject]);

    const updateCursorPosition = useCallback((e) => {
        if (client && timelineRef.current) {
            const rect = timelineRef.current.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            const track = Math.floor(y / TRACK_HEIGHT);
            const time = x / PIXELS_PER_SECOND;

            try {
                client.update_cursor_position(track, time);
            } catch (error) {
                console.error("Failed to update cursor position:", error);
            }
        }
    }, [client]);

    const handleClipMouseDown = useCallback((e, clip) => {
        if (e.button === 0) { // Left mouse button
            if (e.target.classList.contains('resize-handle')) {
                setResizingClip({ clip, initialX: e.clientX });
            } else {
                setDraggingClip({ clip, initialX: e.clientX, initialY: e.clientY });
            }
        }
    }, []);

    const handleMouseMove = useCallback((e) => {
        updateCursorPosition(e);

        if (draggingClip) {
            const dx = e.clientX - draggingClip.initialX;
            const dy = e.clientY - draggingClip.initialY;
            const newStartTime = draggingClip.clip.start_time + dx / PIXELS_PER_SECOND;
            const newTrack = Math.max(0, Math.min(2, draggingClip.clip.track + Math.round(dy / TRACK_HEIGHT)));

            setProject(prevProject => ({
                ...prevProject,
                clips: prevProject.clips.map(c =>
                    c.id === draggingClip.clip.id
                        ? {
                            ...c,
                            start_time: { secs: newStartTime, nanos: 0 },
                            end_time: { secs: newStartTime + (c.end_time.secs - c.start_time.secs), nanos: 0 },
                            track: newTrack
                        }
                        : c
                )
            }));
        } else if (resizingClip) {
            const dx = e.clientX - resizingClip.initialX;
            const newEndTime = resizingClip.clip.end_time.secs + dx / PIXELS_PER_SECOND;

            setProject(prevProject => ({
                ...prevProject,
                clips: prevProject.clips.map(c =>
                    c.id === resizingClip.clip.id
                        ? { ...c, end_time: { secs: Math.max(c.start_time.secs + 1, newEndTime), nanos: 0 } }
                        : c
                )
            }));
        }
    }, [draggingClip, resizingClip, updateCursorPosition]);

    const handleMouseUp = useCallback(() => {
        if (client) {
            if (draggingClip) {
                const updatedClip = project.clips.find(c => c.id === draggingClip.clip.id);
                try {
                    client.move_clip(updatedClip.id, updatedClip.start_time.secs, updatedClip.track);
                    updateProject(client);
                } catch (error) {
                    console.error("Failed to move clip:", error);
                }
            } else if (resizingClip) {
                const updatedClip = project.clips.find(c => c.id === resizingClip.clip.id);
                try {
                    client.resize_clip(updatedClip.id, updatedClip.end_time.secs);
                    updateProject(client);
                } catch (error) {
                    console.error("Failed to resize clip:", error);
                }
            }
        }
        setDraggingClip(null);
        setResizingClip(null);
    }, [client, draggingClip, resizingClip, project.clips, updateProject]);

    const handleTimeUpdate = (newTime) => {
        setCurrentTime(newTime);
    };

    const togglePlayPause = () => {
        setIsPlaying(prevIsPlaying => !prevIsPlaying);
    };

    useEffect(() => {
        if (isPlaying) {
            playIntervalRef.current = setInterval(() => {
                setCurrentTime(prev => {
                    const newTime = prev + 0.1;
                    return newTime >= project.duration.secs ? 0 : newTime;
                });
            }, 100);
        } else if (playIntervalRef.current) {
            clearInterval(playIntervalRef.current);
        }

        return () => {
            if (playIntervalRef.current) {
                clearInterval(playIntervalRef.current);
            }
        };
    }, [isPlaying, project.duration.secs]);

    const handleScrubberMouseDown = useCallback((e) => {
        const rect = timelineRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left;
        setCurrentTime(x / PIXELS_PER_SECOND);
    }, []);

    const applyEffect = useCallback((clipId, effectType, value) => {
        console.log(`Attempting to apply effect: ${effectType} with value ${value} to clip ${clipId}`);
        if (client) {
            try {
                client.apply_effect(clipId, effectType, value);
                console.log('Effect applied successfully, updating project');
                updateProject(client);
            } catch (error) {
                console.error("Failed to apply effect:", error);
            }
        } else {
            console.error("Client is not initialized");
        }
    }, [client, updateProject]);

    const renderTimeline = () => {
        return (
            <div className="tracks" style={{ height: `${3 * TRACK_HEIGHT}px` }}>
                {[0, 1, 2].map(trackIndex => (
                    <div key={`track-${trackIndex}`} className="track" style={{ height: `${TRACK_HEIGHT}px` }}>
                        {project.clips.filter(clip => clip.track === trackIndex).map((clip, index) => (
                            <div
                                key={`${clip.id}-${index}`}
                                className="clip"
                                onMouseDown={(e) => handleClipMouseDown(e, clip)}
                                style={{
                                    position: 'absolute',
                                    top: `${clip.track * TRACK_HEIGHT}px`,
                                    left: `${clip.start_time * PIXELS_PER_SECOND}px`,
                                    width: `${(clip.end_time - clip.start_time) * PIXELS_PER_SECOND}px`,
                                    height: `${TRACK_HEIGHT - 2}px`,
                                    backgroundColor: 'lightblue',
                                    border: '1px solid blue',
                                    cursor: 'move',
                                }}
                            >
                                {clip.id}
                                <div className="resize-handle" style={{
                                    position: 'absolute',
                                    right: 0,
                                    top: 0,
                                    bottom: 0,
                                    width: '5px',
                                    backgroundColor: 'blue',
                                    cursor: 'ew-resize',
                                }} />
                                <button onClick={() => applyEffect(clip.id, 'brightness', 1.2)}>+Bright</button>
                                <button onClick={() => applyEffect(clip.id, 'brightness', 0.8)}>-Bright</button>
                                <button onClick={() => applyEffect(clip.id, 'contrast', 1.2)}>+Contrast</button>
                                <button onClick={() => applyEffect(clip.id, 'contrast', 0.8)}>-Contrast</button>
                                <button onClick={() => applyEffect(clip.id, 'saturation', 1.2)}>+Saturation</button>
                                <button onClick={() => applyEffect(clip.id, 'saturation', 0.8)}>-Saturation</button>
                                <button onClick={() => applyEffect(clip.id, 'hue', 30)}>+Hue</button>
                                <button onClick={() => applyEffect(clip.id, 'hue', -30)}>-Hue</button>
                                <button onClick={() => applyEffect(clip.id, 'grayscale', 1)}>Grayscale</button>
                                <button onClick={() => applyEffect(clip.id, 'grayscale', 0)}>Color</button>
                            </div>
                        ))}
                    </div>
                ))}
                <div className="playhead" style={{
                    position: 'absolute',
                    top: 0,
                    left: `${currentTime * PIXELS_PER_SECOND}px`,
                    width: '2px',
                    height: '100%',
                    backgroundColor: 'red',
                    pointerEvents: 'none',
                }} />
            </div>
        );
    };

    return (
        <div>
            <h1>WeFrame Video Editor</h1>
            <div className="toolbar">
                <button onClick={addClip}>Add Clip</button>
                <button onClick={togglePlayPause}>{isPlaying ? 'Pause' : 'Play'}</button>
            </div>
            <VideoPreview
                currentTime={currentTime}
                clips={project.clips}
                onTimeUpdate={handleTimeUpdate}
                isPlaying={isPlaying}
            />
            <div
                className="timeline-container"
                ref={timelineRef}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseUp}
                onMouseDown={handleScrubberMouseDown}
                style={{ position: 'relative', height: '300px', border: '1px solid black', overflowX: 'auto', overflowY: 'hidden' }}
            >
                {renderTimeline()}
            </div>
            <div className="collaborators">
                {project.collaborators && project.collaborators.map(collaborator => (
                    <div key={collaborator.id} className="collaborator">
                        {collaborator.name} - Track: {collaborator.cursor_position.track}, Time: {collaborator.cursor_position.time.secs}s
                    </div>
                ))}
            </div>
        </div>
    );
};

export default VideoEditor;