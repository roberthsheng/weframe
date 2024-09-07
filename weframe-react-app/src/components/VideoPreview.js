import React, { useRef, useEffect, useState } from 'react';

const VideoPreview = ({ currentTime, clips, onTimeUpdate, isPlaying }) => {
    const videoRef = useRef(null);
    const [activeClip, setActiveClip] = useState(null);
    const [error, setError] = useState(null);

    useEffect(() => {
        const clip = clips.find(c =>
            currentTime >= c.start_time.secs && currentTime < c.end_time.secs
        );
        setActiveClip(clip);
        setError(null);
    }, [currentTime, clips]);

    useEffect(() => {
        if (videoRef.current && activeClip) {
            if (activeClip.source_file) {
                videoRef.current.src = activeClip.source_file;
                videoRef.current.currentTime = currentTime - activeClip.start_time.secs;
                if (isPlaying) {
                    videoRef.current.play().catch(e => {
                        console.error("Error playing video:", e);
                        setError("Failed to play video");
                    });
                } else {
                    videoRef.current.pause();
                }
            } else {
                setError("No video source available for this clip");
            }
        }
    }, [activeClip, currentTime, isPlaying]);

    const handleTimeUpdate = () => {
        if (videoRef.current && activeClip) {
            const newTime = activeClip.start_time.secs + videoRef.current.currentTime;
            onTimeUpdate(newTime);
        }
    };

    const applyEffects = () => {
        if (videoRef.current && activeClip && activeClip.effects) {
            let filterString = '';
            activeClip.effects.forEach(effect => {
                switch (effect.effect_type) {
                    case 'brightness':
                        filterString += `brightness(${effect.value}) `;
                        break;
                    case 'contrast':
                        filterString += `contrast(${effect.value}) `;
                        break;
                    case 'saturation':
                        filterString += `saturate(${effect.value}) `;
                        break;
                    case 'hue':
                        filterString += `hue-rotate(${effect.value}deg) `;
                        break;
                    case 'grayscale':
                        filterString += `grayscale(${effect.value}) `;
                        break;
                }
            });
            videoRef.current.style.filter = filterString;
        }
    };

    useEffect(applyEffects, [activeClip]);

    return (
        <div className="video-preview">
            <video
                ref={videoRef}
                onTimeUpdate={handleTimeUpdate}
                style={{ width: '100%', maxHeight: '300px' }}
            />
            {error && <div className="error">{error}</div>}
            {!activeClip && <div className="no-clip">No active clip</div>}
        </div>
    );
};

export default VideoPreview;