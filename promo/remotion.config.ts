import {Config} from '@remotion/cli/config';

// H.264, yuv420p, CRF 18. scripts/finish.sh solo remuxa con +faststart.
Config.setVideoImageFormat('jpeg');
Config.setJpegQuality(95);
Config.setCodec('h264');
Config.setCrf(18);
Config.setPixelFormat('yuv420p');
Config.setConcurrency(4);
Config.setOverwriteOutput(true);
