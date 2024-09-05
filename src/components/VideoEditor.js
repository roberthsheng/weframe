// src/components/VideoEditor.js
import React, { useState, useEffect } from 'react';
import init, { WeframeClient } from 'weframe-client';

const VideoEditor = () => {
    const [client, setClient] = useState(null);
    const [project, setProject] = useState({ clips: [], duration: { secs: 300, nanos: 0 } });

    useEffect(() => {
        init().then(() => {
            const newClient = new WeframeClient('ws://localhost:3030/ws/default-session');
            setClient(newClient);
            updateProject(newClient);
        }).catch(console.error);
    }, []);

    const updateProject = (clientInstance) => {
        const projectData = clientInstance.get_project();
        setProject(projectData);
    };

    const addClip = () => {
        if (client) {
            const newClip = {
                id: Math.random().toString(36).substr(2, 9),
                start_time: { secs: 0, nanos: 0 },
                end_time: { secs: 10, nanos: 0 },
                track: project.clips.length,
            };
            const operation = {
                client_id: 1, // This should be dynamically assigned
                client_version: 1, // This should be incremented
                server_version: 0, // This should be updated based on server responses
                operation: { AddClip: newClip },
            };
            try {
                client.send_operation(operation);
                updateProject(client);
            } catch (error) {
                console.error("Failed to send operation:", error);
            }
        }
    };

    return (
        <div>
            <h1>WeFrame Video Editor</h1>
            <button onClick={addClip}>Add Clip</button>
            <div>
                {project.clips.map((clip) => (
                    <div key={clip.id}>
                        Clip {clip.id} on track {clip.track}
                    </div>
                ))}
            </div>
        </div>
    );
};

export default VideoEditor;