import React, { useState, useEffect, useCallback, useRef } from 'react';
import init, { WeframeClient } from 'weframe-client';

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
    const timelineRef = useRef(null);

    useEffect(() => {
        init().then(() => {
            const newClient = new WeframeClient('ws://localhost:3030/ws/default-session', 'user1', 'User 1');
            setClient(newClient);
            updateProject(newClient);
        }).catch(console.error);
    }, []);

    const updateProject = useCallback((clientInstance) => {
        const projectData = clientInstance.get_project();
        console.log('Received project data:', projectData);
        setProject(projectData);
    }, []);

    const addClip = useCallback(() => {
        if (client) {
            const newClip = {
                id: Math.random().toString(36).substr(2, 9),
                source_file: "example.mp4",
                start_time: { secs: 0, nanos: 0 },
                end_time: { secs: 10, nanos: 0 },
                track: project.clips.length % 3, // Use multiple tracks
                effects: [],
                transition: null,
            };
            const operation = {
                client_id: "user1",
                client_version: project.clips.length + 1,
                server_version: 0,
                operation: { AddClip: newClip },
            };
            try {
                client.send_operation(operation);
                setProject(prevProject => ({
                    ...prevProject,
                    clips: [...prevProject.clips, newClip]
                }));
            } catch (error) {
                console.error("Failed to send operation:", error);
            }
        }
    }, [client, project.clips]);

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
            const newStartTime = draggingClip.clip.start_time.secs + dx / PIXELS_PER_SECOND;
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
        if (draggingClip || resizingClip) {
            // Send update operation to server
            // This is a simplified version, you'll need to implement the actual operation
            const updatedClip = project.clips.find(c => c.id === (draggingClip?.clip.id || resizingClip?.clip.id));
            const operation = {
                client_id: "user1",
                client_version: project.clips.length,
                server_version: 0,
                operation: {
                    MoveClip: {
                        id: updatedClip.id,
                        new_start_time: updatedClip.start_time,
                        new_track: updatedClip.track
                    }
                },
            };
            client.send_operation(operation);
        }
        setDraggingClip(null);
        setResizingClip(null);
    }, [client, draggingClip, resizingClip, project.clips]);

    const renderTimeline = () => {
        return (
            <div className="tracks" style={{ height: `${3 * TRACK_HEIGHT}px` }}>
                {[0, 1, 2].map(trackIndex => (
                    <div key={trackIndex} className="track" style={{ height: `${TRACK_HEIGHT}px` }}>
                        {project.clips.filter(clip => clip.track === trackIndex).map((clip) => (
                            <div
                                key={clip.id}
                                className="clip"
                                onMouseDown={(e) => handleClipMouseDown(e, clip)}
                                style={{
                                    position: 'absolute',
                                    top: `${clip.track * TRACK_HEIGHT}px`,
                                    left: `${clip.start_time.secs * PIXELS_PER_SECOND}px`,
                                    width: `${(clip.end_time.secs - clip.start_time.secs) * PIXELS_PER_SECOND}px`,
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
                            </div>
                        ))}
                    </div>
                ))}
            </div>
        );
    };

    return (
        <div>
            <h1>WeFrame Video Editor</h1>
            <div className="toolbar">
                <button onClick={addClip}>Add Clip</button>
            </div>
            <div
                className="timeline-container"
                ref={timelineRef}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseUp}
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