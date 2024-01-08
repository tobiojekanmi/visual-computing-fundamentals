#version 430 core

in layout(location = 0) vec4 vertexColor;
in layout(location = 1) vec3 vertexNormal;

out vec4 color;

void main()
{   
    // Implement the Lambertian shading model
    vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));    
    vec4 finalColor = vertexColor * max(0, dot(vertexNormal, -lightDirection));
    color = vec4(finalColor.rgb, vertexColor.a);
}
