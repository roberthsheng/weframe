// VideoPreview.js
import React, { useRef, useEffect, useState, useCallback } from 'react';

const VideoPreview = ({ currentTime, clips, onTimeUpdate, isPlaying }) => {
    const videoRef = useRef(null);
    const [activeClip, setActiveClip] = useState(null);
    const [error, setError] = useState(null);
    const [isLoading, setIsLoading] = useState(false);
    const [videoQueue, setVideoQueue] = useState([]);
    const [isVideoReady, setIsVideoReady] = useState(false);

    const findActiveClip = useCallback((time) => {
        return clips.find(c => time >= c.start_time.secs && time < c.end_time.secs);
    }, [clips]);

    useEffect(() => {
        const newActiveClip = findActiveClip(currentTime);
        if (newActiveClip && (!activeClip || newActiveClip.id !== activeClip.id)) {
            console.log('New active clip:', newActiveClip);
            setActiveClip(newActiveClip);
            setVideoQueue(prevQueue => [...prevQueue, newActiveClip]);
        }
    }, [currentTime, clips, activeClip, findActiveClip]);

    const loadVideo = useCallback(async (clip) => {
        if (!videoRef.current) return;

        setIsLoading(true);
        setIsVideoReady(false);
        setError(null);

        try {
            videoRef.current.src = clip.source_file;
            await videoRef.current.load();
            videoRef.current.currentTime = currentTime - clip.start_time.secs;
            setIsVideoReady(true);
            console.log('Video loaded successfully');
        } catch (e) {
            console.error("Error loading video:", e);
            setError(`Failed to load video: ${e.message}`);
        } finally {
            setIsLoading(false);
        }
    }, [currentTime]);

    const playVideo = useCallback(async () => {
        if (!videoRef.current || !isVideoReady) return;

        try {
            await videoRef.current.play();
            console.log('Video started playing');
        } catch (e) {
            console.error("Error playing video:", e);
            setError(`Failed to play video: ${e.message}`);
        }
    }, [isVideoReady]);

    useEffect(() => {
        const processVideoQueue = async () => {
            if (videoQueue.length > 0 && !isLoading) {
                const nextClip = videoQueue[0];
                await loadVideo(nextClip);
                setVideoQueue(prevQueue => prevQueue.slice(1));
            }
        };

        processVideoQueue();
    }, [videoQueue, isLoading, loadVideo]);

    useEffect(() => {
        if (isPlaying && isVideoReady && !isLoading) {
            playVideo();
        } else if (!isPlaying && videoRef.current) {
            videoRef.current.pause();
            console.log('Video paused');
        }
    }, [isPlaying, isVideoReady, isLoading, playVideo]);

    const handleTimeUpdate = () => {
        if (videoRef.current && activeClip) {
            const newTime = activeClip.start_time.secs + videoRef.current.currentTime;
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
                switch (effect.effect_type) {
                    case 'Brightness':
                        filterString += `brightness(${value}) `;
                        break;
                    case 'Contrast':
                        filterString += `contrast(${value}) `;
                        break;
                    case 'Saturation':
                        filterString += `saturate(${value}) `;
                        break;
                    case 'Hue':
                        filterString += `hue-rotate(${value}deg) `;
                        break;
                    case 'Grayscale':
                        filterString += `grayscale(${value}) `;
                        break;
                    default:
                        console.warn(`Unknown effect type: ${effect.effect_type}`);
                        break;
                }
            });
            console.log('Applying filter:', filterString);
            videoRef.current.style.filter = filterString;
        } else {
            console.log('No active clip or video element');
        }
    }, [activeClip]);

    useEffect(() => {
        applyEffects();
    }, [activeClip, applyEffects]);

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