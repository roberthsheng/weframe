import React, { useRef, useEffect, useState, useCallback } from 'react';

const VideoPreview = ({ currentTime, clips, onTimeUpdate, isPlaying }) => {
    const videoRef = useRef(null);
    const [activeClip, setActiveClip] = useState(null);
    const [error, setError] = useState(null);
    const [isLoading, setIsLoading] = useState(false);
    const [isVideoReady, setIsVideoReady] = useState(false);

    const findActiveClip = useCallback((time) => {
        return clips.find(c => time >= c.start_time && time < c.end_time);
    }, [clips]);

    const loadVideo = useCallback(async (clip) => {
        if (!videoRef.current) return;

        setIsLoading(true);
        setIsVideoReady(false);
        setError(null);

        try {
            videoRef.current.src = clip.source_file;
            await videoRef.current.load();
            videoRef.current.currentTime = currentTime - clip.start_time;
            setIsVideoReady(true);
            console.log('Video loaded successfully');
            if (isPlaying) {
                await videoRef.current.play();
            }
        } catch (e) {
            console.error("Error loading video:", e);
            setError(`Failed to load video: ${e.message}`);
        } finally {
            setIsLoading(false);
        }
    }, [currentTime, isPlaying]);

    useEffect(() => {
        if (activeClip && videoRef.current) {
            if (isPlaying && isVideoReady) {
                videoRef.current.play().catch(e => console.error("Error playing video:", e));
            } else {
                videoRef.current.pause();
                console.log('Video paused');
            }
        }
    }, [isPlaying, isVideoReady, activeClip]);

    const handleTimeUpdate = () => {
        if (videoRef.current && activeClip) {
            const newTime = activeClip.start_time + videoRef.current.currentTime;
            onTimeUpdate(newTime);
        }
    };

    const applyEffects = useCallback(() => {
        if (videoRef.current && activeClip) {
            console.log('Applying effects to clip:', activeClip.id);
            console.log('Effects:', activeClip.effects);
            let filterString = '';
            activeClip.effects.forEach(effect => {
                const value = effect.parameters.value;
                console.log(`Applying effect: ${effect.effect_type}, value: ${value}`);
                // ... rest of the function
            });
            console.log('Applying filter:', filterString);
            videoRef.current.style.filter = filterString;
        } else {
            console.log('No active clip or video element');
        }
    }, [activeClip]);

    useEffect(() => {
        const newActiveClip = findActiveClip(currentTime);
        if (newActiveClip && (!activeClip || newActiveClip.id !== activeClip.id)) {
            console.log('New active clip:', newActiveClip);
            setActiveClip(newActiveClip);
            loadVideo(newActiveClip);
            applyEffects();
        }
    }, [currentTime, clips, activeClip, findActiveClip, loadVideo, applyEffects]);

    return (
        <div className="video-preview">
            {isLoading && <div className="loading">Loading video...</div>}
            <video
                ref={videoRef}
                onTimeUpdate={handleTimeUpdate}
                style={{ width: '100%', maxHeight: '300px', display: isVideoReady ? 'block' : 'none' }}
                onError={(e) => {
                    console.error('Video error:', e);
                    setError(`Video error: ${e.target.error?.message || 'Unknown error'}`);
                }}
            />
            {error && <div className="error">{error}</div>}
            {!activeClip && <div className="no-clip">No active clip</div>}
            <div>
                <h3>Current Clip:</h3>
                <pre>{JSON.stringify(activeClip, null, 2)}</pre>
            </div>
        </div>
    );
};

export default VideoPreview;